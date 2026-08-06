//! The layout macros: `#[bitpanel]` for bit-packed panels, `#[bitbody]` for the
//! byte-segmented bodies that hold them. Each is the single statement of its
//! layout — codec, docs and metadata all generate from the one declaration.
//!
//! # `#[bitpanel]` — a bit-packed panel as an ordinary Rust struct
//!
//! ```ignore
//! /// The attribute's argument is the panel's length in bytes.
//! #[bitpanel(7)]
//! #[derive(Default)]
//! pub struct CenterPanel {
//!     #[bits(0..=2)]
//!     pub left_part: Instrument,
//!     #[bits(35..=41)]
//!     pub gain: Level,
//!     /// Contiguous over the bytes, whatever it looks like in a hex dump.
//!     #[bits(61..=67)]
//!     pub equalizer_freq_gain: Level,
//! }
//! ```
//!
//! Bits are numbered MSB-first from byte 0 of the panel, as `nord-format`'s `bits` module
//! describes. Generates both directions of the conversion between the panel and its
//! `[u8; N]`, a `Debug` over the decoded fields, and a `Panel` impl that lists them,
//! describes them, and sets them by name. Bits no field claims are preserved, and are
//! reported in the panel's generated doc; two ranges may not overlap.
//!
//! `Panel::field_values` and `Panel::field_specs` are emitted in declaration order and
//! describe the same fields, so callers may zip them positionally.
//!
//! Encoding is total. Every field's type has to carry its own range, so the fit is proven
//! at compile time: a raw `u8` in a 7-bit slot fails to compile.
//!
//! # `#[bitbody]` — a container body as a map of byte segments
//!
//! ```ignore
//! /// The attribute's argument is the body's length in bytes.
//! #[bitbody(121)]
//! pub struct ProgramBody {
//!     #[at(0x00..0x02, be)]
//!     program_version: u16,
//!     #[at(0x02..0x09)]
//!     pub center_panel: CenterPanel,
//!     #[at(0x09..0x0e)]
//!     pad1: [u8; 5],
//! }
//! ```
//!
//! Byte ranges are half-open and body-relative, like slice indexes. A segment's
//! type decides its codec: a `[u8; N]` is kept verbatim (padding, unmapped ranges),
//! an unsigned integer takes a `be`/`le` marker, and anything else is a panel —
//! `TryFrom<[u8; N]>` in, `From<&T> -> [u8; N]` out, the pair `#[bitpanel]` emits.
//!
//! Generates the `[u8; LEN]` conversions both ways, the `cbin::Body` impl, a
//! `Debug` over the non-verbatim fields, a layout table in the struct's docs, and
//! a `layout::BodyLayout` impl exposing the segments as data — panel segments
//! chain into their `FieldSpec`s, so the file-offset-to-bit map is one walk.
//!
//! **Segments are declared in file order and must cover every byte exactly
//! once.** A gap, an overlap, or a wrong total is a compile error, not a shifted
//! decode: unmapped bytes are spelled as `[u8; N]` segments so a re-encode keeps
//! them.
//!
//! Both macros are only usable inside `nord-format`: generated code names
//! `crate::bits`, `crate::cbin`, `crate::error`, `crate::layout` and `crate::panel`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Expr, ExprRange, Ident, ItemStruct, Lit, LitInt, RangeLimits, Type};

/// One field's `LO..=HI` placement, MSB-first from byte 0 of the panel.
struct Bits {
    lo: u32,
    hi: u32,
}

impl syn::parse::Parse for Bits {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let range: ExprRange = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("a field has one `LO..=HI` placement"));
        }

        if !matches!(range.limits, RangeLimits::Closed(_)) {
            return Err(syn::Error::new_spanned(
                &range,
                "bit ranges are inclusive: write `0..=2`, not `0..2`",
            ));
        }

        let lo = literal(range.start.as_deref(), &range, "low bit")?;
        let hi = literal(range.end.as_deref(), &range, "high bit")?;

        if hi < lo {
            return Err(syn::Error::new_spanned(
                &range,
                format!("bit range ends before it starts: `{lo}..={hi}`"),
            ));
        }

        Ok(Bits { lo, hi })
    }
}

fn literal(expr: Option<&Expr>, at: &ExprRange, what: &str) -> syn::Result<u32> {
    match expr {
        Some(Expr::Lit(lit)) => match &lit.lit {
            Lit::Int(int) => int.base10_parse(),
            other => Err(syn::Error::new_spanned(
                other,
                format!("{what} must be an integer"),
            )),
        },
        _ => Err(syn::Error::new_spanned(
            at,
            format!("{what} is missing: a bit range needs both ends"),
        )),
    }
}

/// `Bits 24..=27 (byte 0x03, bits 7..4).` — the range as a hex dump reads it.
fn breakdown(lo: u32, hi: u32) -> String {
    let mut parts = Vec::new();
    let mut at = lo;
    while at <= hi {
        let byte = at / 8;
        let last = hi.min(byte * 8 + 7);
        let (first_bit, last_bit) = (7 - at % 8, 7 - last % 8);
        parts.push(if first_bit == last_bit {
            format!("byte {byte:#04x}, bit {first_bit}")
        } else {
            format!("byte {byte:#04x}, bits {first_bit}..{last_bit}")
        });
        at = last + 1;
    }
    format!("Bits {lo}..={hi} ({}).", parts.join("; "))
}

/// The ranges of `0..bits` no field claims.
fn unclaimed(claimed: &[(u32, u32)], bits: u32) -> Vec<(u32, u32)> {
    let mut sorted = claimed.to_vec();
    sorted.sort_unstable();

    let mut gaps = Vec::new();
    let mut next = 0;
    for (lo, hi) in sorted {
        if lo > next {
            gaps.push((next, lo - 1));
        }
        next = hi + 1;
    }
    if next < bits {
        gaps.push((next, bits - 1));
    }
    gaps
}

#[proc_macro_attribute]
pub fn bitpanel(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let len: LitInt = syn::parse2(attr.clone()).map_err(|_| {
        syn::Error::new(
            attr.span(),
            "expected the panel's length in bytes — e.g. `#[bitpanel(7)]`",
        )
    })?;
    let bytes: usize = len.base10_parse()?;
    if bytes == 0 {
        return Err(syn::Error::new_spanned(&len, "a panel needs a byte"));
    }
    let span_bits = (bytes * 8) as u32;

    let panel: ItemStruct = syn::parse2(item)?;
    let name = &panel.ident;
    let vis = &panel.vis;
    let attrs = &panel.attrs;

    let syn::Fields::Named(named) = &panel.fields else {
        return Err(syn::Error::new_spanned(
            &panel,
            "a panel must be a struct with named fields",
        ));
    };

    let mut claimed: Vec<(u32, u32, &Ident)> = Vec::new();

    let mut decode = Vec::new();
    let mut encode = Vec::new();
    let mut debug = Vec::new();
    let mut fields = Vec::new();
    let mut values = Vec::new();
    let mut specs = Vec::new();
    let mut setters = Vec::new();

    for field in &named.named {
        let ident = field.ident.as_ref().expect("named fields");
        let ty = &field.ty;

        let attr = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident("bits"))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    field,
                    "every field of a panel needs a `#[bits(LO..=HI)]` placement",
                )
            })?;
        let Bits { lo, hi } = attr.parse_args()?;

        if hi >= span_bits {
            return Err(syn::Error::new_spanned(
                attr,
                format!(
                    "bit {hi} is past the end of a {bytes}-byte panel, whose last bit is {}",
                    span_bits - 1,
                ),
            ));
        }
        if let Some(&(olo, ohi, other)) = claimed.iter().find(|&&(l, h, _)| lo <= h && l <= hi) {
            return Err(syn::Error::new_spanned(
                attr,
                format!("bits {lo}..={hi} overlap `{other}`, at {olo}..={ohi}"),
            ));
        }
        claimed.push((lo, hi, ident));

        // Keep everything but the placement, which has served its purpose.
        let kept: Vec<_> = field
            .attrs
            .iter()
            .filter(|a| !a.path().is_ident("bits"))
            .collect();
        let placement_doc = breakdown(lo, hi);
        let field_vis = &field.vis;
        fields.push(quote! {
            #(#kept)*
            #[doc = ""]
            #[doc = #placement_doc]
            #field_vis #ident: #ty
        });

        let f = quote! { crate::bits::Field::<#ty, #lo, #hi> };
        decode.push(quote! { #ident: #f::get(&raw)? });
        encode.push(quote! { #f::set(&mut raw, p.#ident); });

        // The field's bits with no type applied — see `crate::panel::FieldValue`.
        let placement = format!("{lo}..={hi}");
        values.push(quote! {
            crate::panel::FieldValue {
                name: stringify!(#ident),
                placement: #placement,
                raw: crate::bits::Field::<u64, #lo, #hi>::read(&self.raw),
                bits: <#ty as crate::bits::Packed>::to_bits(&self.#ident),
                value: ::std::format!("{:?}", &self.#ident),
            }
        });

        let width = hi - lo + 1;
        specs.push(quote! {
            crate::panel::FieldSpec {
                name: stringify!(#ident),
                placement: #placement,
                width: #width,
                legal: || crate::panel::legal_values::<#ty>(#width),
            }
        });

        // The parse is the type's, so a value it cannot hold fails here rather than
        // being clamped into the slot.
        setters.push(quote! {
            stringify!(#ident) => {
                self.#ident = crate::panel::parse_field::<#ty>(#width, value)
                    .map_err(|e| e.at(stringify!(#ident)))?;
                Ok(())
            }
        });

        debug.push(quote! { .field(stringify!(#ident), &self.#ident) });
    }

    let gaps = unclaimed(
        &claimed.iter().map(|&(l, h, _)| (l, h)).collect::<Vec<_>>(),
        span_bits,
    );
    let gap_doc = if gaps.is_empty() {
        format!("Every one of the panel's {span_bits} bits is named.")
    } else {
        format!(
            "Unclaimed bits, kept verbatim through a re-encode: {}.",
            gaps.iter()
                .map(|&(l, h)| if l == h {
                    format!("{l}")
                } else {
                    format!("{l}..={h}")
                })
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    Ok(quote! {
        #(#attrs)*
        #[doc = ""]
        #[doc = #gap_doc]
        #vis struct #name {
            /// The bytes this panel was decoded from, so bits no field claims survive a
            /// re-encode. Named fields take precedence on write.
            raw: [u8; #bytes],
            #(#fields,)*
        }

        impl ::core::convert::TryFrom<[u8; #bytes]> for #name {
            type Error = crate::error::ParseError;

            fn try_from(raw: [u8; #bytes]) -> ::core::result::Result<Self, Self::Error> {
                Ok(#name { raw, #(#decode,)* })
            }
        }

        impl ::core::convert::From<&#name> for [u8; #bytes] {
            fn from(p: &#name) -> Self {
                let mut raw = p.raw;
                #(#encode)*
                raw
            }
        }

        impl crate::panel::Panel for #name {
            const NAME: &'static str = stringify!(#name);

            fn field_values(&self) -> ::std::vec::Vec<crate::panel::FieldValue> {
                ::std::vec![#(#values,)*]
            }

            fn field_specs() -> ::std::vec::Vec<crate::panel::FieldSpec> {
                ::std::vec![#(#specs,)*]
            }

            fn set_field(
                &mut self,
                name: &str,
                value: &str,
            ) -> ::core::result::Result<(), crate::panel::FieldError> {
                match name {
                    #(#setters)*
                    other => Err(crate::panel::FieldError::UnknownField {
                        panel: <Self as crate::panel::Panel>::NAME,
                        name: other.to_string(),
                    }),
                }
            }
        }

        /// The decoded values; the backing bytes are not printed.
        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #(#debug)*
                    .finish()
            }
        }
    })
}

#[proc_macro_attribute]
pub fn bitbody(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_body(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// One segment's `#[at(LO..HI)]` placement, with the endian marker for integers.
struct At {
    start: usize,
    end: usize,
    /// `Some(true)` for `be`, `Some(false)` for `le`, `None` when unmarked.
    big_endian: Option<bool>,
}

impl syn::parse::Parse for At {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let range: ExprRange = input.parse()?;
        if !matches!(range.limits, RangeLimits::HalfOpen(_)) {
            return Err(syn::Error::new_spanned(
                &range,
                "byte ranges are half-open, like slice indexes: write `0x02..0x09`",
            ));
        }
        let start = byte_literal(range.start.as_deref(), &range, "start byte")?;
        let end = byte_literal(range.end.as_deref(), &range, "end byte")?;
        if end <= start {
            return Err(syn::Error::new_spanned(
                &range,
                format!("byte range is empty: `{start:#04x}..{end:#04x}`"),
            ));
        }

        let big_endian = if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            let marker: Ident = input.parse()?;
            match marker.to_string().as_str() {
                "be" => Some(true),
                "le" => Some(false),
                other => {
                    return Err(syn::Error::new_spanned(
                        &marker,
                        format!("unknown marker `{other}`: an integer segment takes `be` or `le`"),
                    ))
                }
            }
        } else {
            None
        };
        if !input.is_empty() {
            return Err(input.error("a segment has one `LO..HI` placement and at most one marker"));
        }
        Ok(At {
            start,
            end,
            big_endian,
        })
    }
}

fn byte_literal(expr: Option<&Expr>, at: &ExprRange, what: &str) -> syn::Result<usize> {
    match expr {
        Some(Expr::Lit(lit)) => match &lit.lit {
            Lit::Int(int) => int.base10_parse(),
            other => Err(syn::Error::new_spanned(
                other,
                format!("{what} must be an integer"),
            )),
        },
        _ => Err(syn::Error::new_spanned(
            at,
            format!("{what} is missing: a byte range needs both ends"),
        )),
    }
}

/// What a segment's type says about its codec.
enum Codec {
    /// `[u8; N]` — bytes kept verbatim.
    Verbatim,
    /// `u8`/`u16`/`u32`/`u64`, `width` bytes wide.
    Uint { width: usize, big_endian: bool },
    /// Anything else: `TryFrom<[u8; N]>` / `From<&T> -> [u8; N]`.
    Panel,
}

/// Classify a segment by its type, checking the type's width against the range's.
fn classify(ty: &Type, at: &At, field: &syn::Field) -> syn::Result<Codec> {
    let span = at.end - at.start;
    if let Type::Array(array) = ty {
        if !matches!(&*array.elem, Type::Path(p) if p.path.is_ident("u8")) {
            return Err(syn::Error::new_spanned(
                ty,
                "a verbatim segment is `[u8; N]`",
            ));
        }
        let Expr::Lit(lit) = &array.len else {
            return Err(syn::Error::new_spanned(&array.len, "spell the length out"));
        };
        let Lit::Int(int) = &lit.lit else {
            return Err(syn::Error::new_spanned(&array.len, "spell the length out"));
        };
        let n: usize = int.base10_parse()?;
        if n != span {
            return Err(syn::Error::new_spanned(
                field,
                format!("`[u8; {n}]` does not fill its {span}-byte range"),
            ));
        }
        if at.big_endian.is_some() {
            return Err(syn::Error::new_spanned(
                field,
                "a verbatim segment has no endianness",
            ));
        }
        return Ok(Codec::Verbatim);
    }

    let width = match ty {
        Type::Path(p) if p.path.is_ident("u8") => Some(1),
        Type::Path(p) if p.path.is_ident("u16") => Some(2),
        Type::Path(p) if p.path.is_ident("u32") => Some(4),
        Type::Path(p) if p.path.is_ident("u64") => Some(8),
        _ => None,
    };
    if let Some(width) = width {
        if width != span {
            return Err(syn::Error::new_spanned(
                field,
                format!("a {width}-byte integer does not fill its {span}-byte range"),
            ));
        }
        let big_endian = match at.big_endian {
            Some(e) => e,
            None if width == 1 => false,
            None => {
                return Err(syn::Error::new_spanned(
                    field,
                    "a multi-byte integer needs its byte order: `#[at(LO..HI, be)]` or `le`",
                ))
            }
        };
        return Ok(Codec::Uint { width, big_endian });
    }

    if at.big_endian.is_some() {
        return Err(syn::Error::new_spanned(
            field,
            "a panel segment has no endianness: the panel owns its bit layout",
        ));
    }
    Ok(Codec::Panel)
}

fn expand_body(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let len_lit: LitInt = syn::parse2(attr.clone()).map_err(|_| {
        syn::Error::new(
            attr.span(),
            "expected the body's length in bytes — e.g. `#[bitbody(121)]`",
        )
    })?;
    let len: usize = len_lit.base10_parse()?;
    if len == 0 {
        return Err(syn::Error::new_spanned(&len_lit, "a body needs a byte"));
    }

    let body: ItemStruct = syn::parse2(item)?;
    let name = &body.ident;
    let vis = &body.vis;
    let attrs = &body.attrs;

    let syn::Fields::Named(named) = &body.fields else {
        return Err(syn::Error::new_spanned(
            &body,
            "a body must be a struct with named fields",
        ));
    };

    let mut cursor = 0usize;
    let mut fields = Vec::new();
    let mut decode = Vec::new();
    let mut encode = Vec::new();
    let mut segments = Vec::new();
    let mut debug = Vec::new();
    let mut table = vec![
        "Body layout (generated). Offsets are body-relative; add `0x2c` (type 1) or \
         `0x18` (type 0) for the file offset a hex dump shows."
            .to_string(),
        String::new(),
        "| bytes | field | holds |".to_string(),
        "|---|---|---|".to_string(),
    ];

    for field in &named.named {
        let ident = field.ident.as_ref().expect("named fields");
        let ty = &field.ty;

        let attr = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident("at"))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    field,
                    "every segment needs an `#[at(LO..HI)]` placement, in file order",
                )
            })?;
        let at: At = attr.parse_args()?;
        let (start, end) = (at.start, at.end);

        if start != cursor {
            return Err(syn::Error::new_spanned(
                attr,
                format!(
                    "segment starts at {start:#04x} where {cursor:#04x} was expected — \
                     segments are declared in file order and cover every byte exactly once; \
                     spell an unmapped range as a `[u8; N]` field",
                ),
            ));
        }
        if end > len {
            return Err(syn::Error::new_spanned(
                attr,
                format!("byte {end:#04x} is past the end of a {len}-byte body"),
            ));
        }
        cursor = end;

        let codec = classify(ty, &at, field)?;
        let n = end - start;

        // Keep everything but the placement, which has served its purpose.
        let kept: Vec<_> = field
            .attrs
            .iter()
            .filter(|a| !a.path().is_ident("at"))
            .collect();
        let placement_doc = format!(
            "Bytes {start:#04x}..{end:#04x} ({:#04x}..{:#04x} in a type-1 file).",
            start + 0x2c,
            end + 0x2c,
        );
        let field_vis = &field.vis;
        fields.push(quote! {
            #(#kept)*
            #[doc = ""]
            #[doc = #placement_doc]
            #field_vis #ident: #ty
        });

        let grab = quote! {{
            let mut b = [0u8; #n];
            b.copy_from_slice(&raw[#start..#end]);
            b
        }};
        let ty_str = quote!(#ty).to_string().replace(' ', "");
        let (holds, kind) = match codec {
            Codec::Verbatim => {
                decode.push(quote! { #ident: #grab });
                encode.push(quote! { raw[#start..#end].copy_from_slice(&p.#ident); });
                (
                    "verbatim".to_string(),
                    quote! { crate::layout::SegmentKind::Verbatim },
                )
            }
            Codec::Uint { width, big_endian } => {
                let (from, to) = if big_endian {
                    (quote!(from_be_bytes), quote!(to_be_bytes))
                } else {
                    (quote!(from_le_bytes), quote!(to_le_bytes))
                };
                decode.push(quote! { #ident: <#ty>::#from(#grab) });
                encode.push(quote! { raw[#start..#end].copy_from_slice(&p.#ident.#to()); });
                debug.push(quote! { .field(stringify!(#ident), &self.#ident) });
                let holds = if width == 1 {
                    format!("`{ty_str}`")
                } else if big_endian {
                    format!("`{ty_str}`, big-endian")
                } else {
                    format!("`{ty_str}`, little-endian")
                };
                (
                    holds,
                    quote! { crate::layout::SegmentKind::Uint { big_endian: #big_endian } },
                )
            }
            Codec::Panel => {
                decode.push(quote! {
                    #ident: <#ty as ::core::convert::TryFrom<[u8; #n]>>::try_from(#grab)?
                });
                encode.push(quote! {
                    raw[#start..#end].copy_from_slice(&<[u8; #n]>::from(&p.#ident));
                });
                debug.push(quote! { .field(stringify!(#ident), &self.#ident) });
                (
                    format!("panel [`{ty_str}`]"),
                    quote! {
                        crate::layout::SegmentKind::Panel {
                            field_specs: || <#ty as crate::panel::Panel>::field_specs(),
                        }
                    },
                )
            }
        };
        table.push(format!(
            "| `{start:#04x}..{end:#04x}` | `{ident}` | {holds} |"
        ));
        segments.push(quote! {
            crate::layout::Segment {
                name: stringify!(#ident),
                start: #start,
                end: #end,
                ty: #ty_str,
                kind: #kind,
            }
        });
    }

    if cursor != len {
        return Err(syn::Error::new_spanned(
            &len_lit,
            format!(
                "segments end at {cursor:#04x} but the body is {len:#04x} bytes — \
                 spell the unmapped tail as a `[u8; N]` field so a re-encode keeps it",
            ),
        ));
    }

    let table: Vec<TokenStream2> = table.iter().map(|row| quote! { #[doc = #row] }).collect();

    Ok(quote! {
        #(#attrs)*
        #[doc = ""]
        #(#table)*
        #vis struct #name {
            #(#fields,)*
        }

        impl ::core::convert::TryFrom<[u8; #len]> for #name {
            type Error = crate::error::Error;

            fn try_from(raw: [u8; #len]) -> ::core::result::Result<Self, Self::Error> {
                Ok(#name { #(#decode,)* })
            }
        }

        impl ::core::convert::From<&#name> for [u8; #len] {
            fn from(p: &#name) -> Self {
                let mut raw = [0u8; #len];
                #(#encode)*
                raw
            }
        }

        impl crate::cbin::Body for #name {
            const LEN: ::core::option::Option<u64> = ::core::option::Option::Some(#len as u64);

            fn read<R: ::std::io::Read + ::std::io::Seek>(
                r: &mut crate::cbin::BodyReader<'_, R>,
                _: &crate::cbin::Header,
            ) -> ::core::result::Result<Self, crate::error::Error> {
                let mut raw = [0u8; #len];
                ::std::io::Read::read_exact(r, &mut raw)?;
                ::core::convert::TryFrom::try_from(raw)
            }

            fn write<W: ::std::io::Write + ::std::io::Seek>(
                &self,
                w: &mut crate::cbin::BodyWriter<'_, W>,
            ) -> ::core::result::Result<(), crate::error::Error> {
                ::std::io::Write::write_all(w, &<[u8; #len]>::from(self))?;
                Ok(())
            }
        }

        impl crate::layout::BodyLayout for #name {
            fn layout() -> &'static [crate::layout::Segment] {
                const SEGMENTS: &[crate::layout::Segment] = &[#(#segments,)*];
                SEGMENTS
            }
        }

        /// The decoded values; verbatim segments are not printed.
        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #(#debug)*
                    .finish()
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_breakdown_reads_like_a_hex_dump() {
        assert_eq!(breakdown(24, 27), "Bits 24..=27 (byte 0x03, bits 7..4).");
        assert_eq!(breakdown(23, 23), "Bits 23..=23 (byte 0x02, bit 0).");
        assert_eq!(
            breakdown(61, 67),
            "Bits 61..=67 (byte 0x07, bits 2..0; byte 0x08, bits 7..4).",
        );
    }

    #[test]
    fn unclaimed_finds_the_holes_and_both_ends() {
        assert_eq!(unclaimed(&[(0, 2), (5, 9)], 16), vec![(3, 4), (10, 15)]);
        assert_eq!(unclaimed(&[(0, 7)], 8), vec![]);
        assert_eq!(unclaimed(&[(4, 7)], 8), vec![(0, 3)]);
    }
}
