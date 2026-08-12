//! `#[bitbody]` — a bit-mapped structure declared once, composable recursively.
//!
//! > This is an unofficial, community project: **not affiliated with, endorsed
//! > by, or supported by Clavia DMI AB**. "Nord" is Clavia's trademark, used
//! > here only to identify the file formats the `nord-format` crate reads.
//!
//! ```ignore
//! /// The attribute's argument is the structure's length in bytes.
//! #[bitbody(121)]
//! pub struct Program {
//!     #[bits(0..=15)]
//!     program_version: u16,
//!     #[at(0x02..0x09)]
//!     pub center_panel: CenterPanel,   // itself a #[bitbody(7)]
//! }
//!
//! #[bitbody(7)]
//! #[derive(Default)]
//! pub struct CenterPanel {
//!     #[bits(0..=2)]
//!     pub lower_part: Instrument,
//! }
//! ```
//!
//! Two placements, one bit space:
//!
//! - `#[bits(LO..=HI)]` — a leaf value at an inclusive bit range, MSB-first from
//!   byte 0, as `nord-format`'s `bits` module describes. The type carries its own
//!   range (`Packed`), so a value wider than its slot fails to compile. A
//!   multi-byte integer leaf is big-endian by construction.
//! - `#[at(LO..HI)]` — a nested `#[bitbody]` at a half-open byte range, placed
//!   via the `TryFrom<[u8; N]>` / `From<&T> -> [u8; N]` pair every bitbody
//!   generates. Nesting is how a large format keeps its real logical layout —
//!   the Electro 5 program *is* five panels — without a second macro for the
//!   inner level.
//!
//! Bits no field claims are preserved verbatim through a re-encode and reported
//! in the generated doc; ranges may not overlap, whichever kind claimed them.
//!
//! The struct's doc carries a bit/byte map: one markdown row per field and per
//! unclaimed run, in offset order, so the rows tile the body. A nested body is
//! one row linking to its own type, whose doc holds the map of its bits.
//!
//! Generates: the `[u8; LEN]` conversions both ways, the `cbin::Body` impl, a
//! `Debug` over the decoded fields, a `layout::BodyLayout` impl publishing every
//! placement as data (nested bodies chain to their own layouts), and — for `pub`
//! fields — the registry: `fields()`, `set_field()`, `field_values()`,
//! `field_specs()`. Private fields decode and encode but stay unregistered.
//!
//! **Paths.** A nested field registers its children under its own name:
//! `center_panel.transpose`. A leaf registers under its own name alone.
//!
//! **Names carry two relations the type cannot.** A registered leaf named `x_wheel`,
//! `x_aftertouch` or `x_ctrl_pedal` beside a registered `x` is that parameter's morph
//! slot, and one named `…_N` is drawbar N of a register. Both are applied to the field's
//! `ControlKind`, which honours whichever it has a use for and ignores the other — so a
//! field named like one of these but typed as something else is unaffected, and a morph
//! slot with no parameter beside it binds to nothing rather than to a guess.
//!
//! Only usable inside `nord-format`: generated code names `crate::bits`,
//! `crate::cbin`, `crate::error`, `crate::layout` and `crate::fields`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Expr, ExprRange, Ident, ItemStruct, Lit, LitInt, RangeLimits};

/// A leaf's `LO..=HI` bit placement.
struct Bits {
    lo: u32,
    hi: u32,
}

impl syn::parse::Parse for Bits {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let range: ExprRange = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("a leaf has one `LO..=HI` placement"));
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

/// A nested body's `LO..HI` byte placement.
struct At {
    start: u32,
    end: u32,
}

impl syn::parse::Parse for At {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let range: ExprRange = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("a nested body has one `LO..HI` placement"));
        }
        if !matches!(range.limits, RangeLimits::HalfOpen(_)) {
            return Err(syn::Error::new_spanned(
                &range,
                "byte ranges are half-open, like slice indexes: write `0x02..0x09`",
            ));
        }
        let start = literal(range.start.as_deref(), &range, "start byte")?;
        let end = literal(range.end.as_deref(), &range, "end byte")?;
        if end <= start {
            return Err(syn::Error::new_spanned(
                &range,
                format!("byte range is empty: `{start:#04x}..{end:#04x}`"),
            ));
        }
        Ok(At { start, end })
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
            format!("{what} is missing: a range needs both ends"),
        )),
    }
}

/// Every byte the range touches, as `(byte, first_bit, last_bit)` — the bit
/// numbers MSB-first within that byte, so they count down.
fn bytes_touched(lo: u32, hi: u32) -> Vec<(u32, u32, u32)> {
    let mut parts = Vec::new();
    let mut at = lo;
    while at <= hi {
        let byte = at / 8;
        let last = hi.min(byte * 8 + 7);
        parts.push((byte, 7 - at % 8, 7 - last % 8));
        at = last + 1;
    }
    parts
}

/// `Bits 24..=27 (byte 0x03, bits 7..4).` — the range as a hex dump reads it.
fn breakdown(lo: u32, hi: u32) -> String {
    let parts: Vec<String> = bytes_touched(lo, hi)
        .into_iter()
        .map(|(byte, first, last)| {
            if first == last {
                format!("byte {byte:#04x}, bit {first}")
            } else {
                format!("byte {byte:#04x}, bits {first}..{last}")
            }
        })
        .collect();
    format!("Bits {lo}..={hi} ({}).", parts.join("; "))
}

/// The range as a hex dump locates it, for a table cell: whole bytes as the
/// half-open byte range `#[at]` writes, anything else byte by byte.
fn hex_span(lo: u32, hi: u32) -> String {
    if lo.is_multiple_of(8) && (hi + 1).is_multiple_of(8) {
        let (start, end) = (lo / 8, (hi + 1) / 8);
        return if end - start == 1 {
            format!("`{start:#04x}`")
        } else {
            format!("`{start:#04x}..{end:#04x}`")
        };
    }
    bytes_touched(lo, hi)
        .into_iter()
        .map(|(byte, first, last)| {
            if first == last {
                format!("`{byte:#04x}` bit {first}")
            } else {
                format!("`{byte:#04x}` bits {first}..{last}")
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// One row of the map: a placed field, or a run of bits no field claims.
struct MapRow {
    lo: u32,
    hi: u32,
    /// The field's name and type, as written; `None` for an unclaimed run.
    field: Option<(String, String)>,
}

/// A plain identifier names a type in scope where the body is declared, so
/// rustdoc resolves a link to it; a generic or a path is left as plain text.
fn ty_cell(ty: &str) -> String {
    let plain = !ty.is_empty()
        && !ty.starts_with(|c: char| c.is_ascii_digit())
        && ty.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if plain {
        format!("[`{ty}`]")
    } else {
        format!("`{ty}`")
    }
}

/// The body's bit map as a markdown table, one line per doc attribute, in
/// offset order. The rows tile the body: every bit is in exactly one of them.
fn map_table(mut rows: Vec<MapRow>) -> Vec<String> {
    rows.sort_by_key(|row| row.lo);
    let mut lines = vec![
        "| bytes | bits | field | type |".to_string(),
        "|---|---|---|---|".to_string(),
    ];
    lines.extend(rows.into_iter().map(|row| {
        let (field, ty) = match &row.field {
            Some((name, ty)) => (format!("`{name}`"), ty_cell(ty)),
            None => ("—".to_string(), "*unclaimed*".to_string()),
        };
        format!(
            "| {} | `{}..={}` | {field} | {ty} |",
            hex_span(row.lo, row.hi),
            row.lo,
            row.hi,
        )
    }));
    lines
}

/// The suffixes a morph slot's name ends in, one per performance control.
const MORPH_SUFFIXES: [&str; 3] = ["_wheel", "_aftertouch", "_ctrl_pedal"];

/// The parameter a slot named `x_wheel` morphs, when the body registers an `x`.
///
/// The convention is the formats' own and it is systematic, so binding here costs one
/// pass over the field list and saves every caller a table of names. A slot whose
/// parameter is not beside it binds to nothing rather than to a guess.
fn morphed_parent<'a>(field: &str, registered: &[&'a str]) -> Option<&'a str> {
    let stem = MORPH_SUFFIXES
        .iter()
        .find_map(|suffix| field.strip_suffix(suffix))?;
    registered.iter().copied().find(|&name| name == stem)
}

/// The drawbar rank a name ending in `_N` declares — 1 is the 16' bar.
///
/// Applied to every leaf and honoured only by a drawbar, so a field that ends in a digit
/// for some other reason keeps whatever its type said.
fn trailing_ordinal(field: &str) -> Option<u8> {
    let stem = field.trim_end_matches(|c: char| c.is_ascii_digit());
    if !stem.ends_with('_') {
        return None;
    }
    field[stem.len()..].parse().ok()
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

/// Declares a bit-mapped structure of `LEN` bytes: `#[bitbody(LEN)]`. The
/// crate-level docs describe the two placement attributes and everything the
/// expansion generates.
#[proc_macro_attribute]
pub fn bitbody(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let len: LitInt = syn::parse2(attr.clone()).map_err(|_| {
        syn::Error::new(
            attr.span(),
            "expected the body's length in bytes — e.g. `#[bitbody(121)]`",
        )
    })?;
    let bytes: usize = len.base10_parse()?;
    if bytes == 0 {
        return Err(syn::Error::new_spanned(&len, "a body needs a byte"));
    }
    let span_bits = (bytes * 8) as u32;

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

    // Every registered field's name, for the one relation a field's own declaration
    // cannot state: which parameter a morph slot belongs to. A private parent would not
    // be in the registry for a caller to resolve, so it is not one.
    let registered: Vec<String> = named
        .named
        .iter()
        .filter(|f| matches!(f.vis, syn::Visibility::Public(_)))
        .filter_map(|f| f.ident.as_ref().map(Ident::to_string))
        .collect();
    let registered: Vec<&str> = registered.iter().map(String::as_str).collect();

    let mut claimed: Vec<(u32, u32, &Ident)> = Vec::new();
    let mut rows: Vec<MapRow> = Vec::new();

    let mut fields = Vec::new();
    let mut decode = Vec::new();
    let mut encode = Vec::new();
    let mut debug = Vec::new();
    let mut values = Vec::new();
    let mut specs = Vec::new();
    let mut setters = Vec::new();
    let mut routes = Vec::new();
    let mut layout = Vec::new();

    for field in &named.named {
        let ident = field.ident.as_ref().expect("named fields");
        let ty = &field.ty;
        let ty_str = quote!(#ty).to_string().replace(' ', "");
        let public = matches!(field.vis, syn::Visibility::Public(_));

        let bits_attr = field.attrs.iter().find(|a| a.path().is_ident("bits"));
        let at_attr = field.attrs.iter().find(|a| a.path().is_ident("at"));

        // The bit span this field claims, and the placement doc, either way.
        let (lo, hi) = match (&bits_attr, &at_attr) {
            (Some(attr), None) => {
                let Bits { lo, hi } = attr.parse_args()?;
                (lo, hi)
            }
            (None, Some(attr)) => {
                let At { start, end } = attr.parse_args()?;
                (start * 8, end * 8 - 1)
            }
            (Some(_), Some(and)) => {
                return Err(syn::Error::new_spanned(
                    and,
                    "one placement per field: `#[bits]` for a leaf or `#[at]` for a nested body",
                ));
            }
            (None, None) => {
                return Err(syn::Error::new_spanned(
                    field,
                    "every field needs a placement: `#[bits(LO..=HI)]` for a leaf, \
                     `#[at(LO..HI)]` for a nested body",
                ));
            }
        };

        if hi >= span_bits {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "bit {hi} is past the end of a {bytes}-byte body, whose last bit is {}",
                    span_bits - 1,
                ),
            ));
        }
        if let Some(&(olo, ohi, other)) = claimed.iter().find(|&&(l, h, _)| lo <= h && l <= hi) {
            return Err(syn::Error::new_spanned(
                field,
                format!("bits {lo}..={hi} overlap `{other}`, at {olo}..={ohi}"),
            ));
        }
        claimed.push((lo, hi, ident));
        rows.push(MapRow {
            lo,
            hi,
            field: Some((ident.to_string(), ty_str.clone())),
        });

        // Keep everything but the placement, which has served its purpose.
        let kept: Vec<_> = field
            .attrs
            .iter()
            .filter(|a| !a.path().is_ident("bits") && !a.path().is_ident("at"))
            .collect();
        let placement_doc = if at_attr.is_some() {
            format!("Bytes {:#04x}..{:#04x}.", lo / 8, (hi + 1) / 8)
        } else {
            breakdown(lo, hi)
        };
        let field_vis = &field.vis;
        fields.push(quote! {
            #(#kept)*
            #[doc = ""]
            #[doc = #placement_doc]
            #field_vis #ident: #ty
        });
        debug.push(quote! { .field(stringify!(#ident), &self.#ident) });

        if at_attr.is_some() {
            // ── a nested body, placed by its own [u8; N] conversions ──
            let (start, end) = ((lo / 8) as usize, ((hi + 1) / 8) as usize);
            let n = end - start;
            decode.push(quote! {
                #ident: <#ty as ::core::convert::TryFrom<[u8; #n]>>::try_from({
                    let mut b = [0u8; #n];
                    b.copy_from_slice(&raw[#start..#end]);
                    b
                })?
            });
            encode.push(quote! {
                raw[#start..#end].copy_from_slice(&<[u8; #n]>::from(&p.#ident));
            });
            layout.push(quote! {
                crate::layout::LayoutField {
                    path: stringify!(#ident),
                    ty: #ty_str,
                    lo: #lo,
                    hi: #hi,
                    nested: ::core::option::Option::Some(
                        <#ty as crate::layout::BodyLayout>::layout,
                    ),
                }
            });
            if public {
                let prefix = format!("{ident}.");
                values.push(quote! {
                    out.extend(self.#ident.field_values().into_iter().map(|mut v| {
                        v.name.insert_str(0, #prefix);
                        v
                    }));
                });
                specs.push(quote! {
                    out.extend(<#ty>::field_specs().into_iter().map(|mut s| {
                        s.name.insert_str(0, #prefix);
                        s
                    }));
                });
                routes.push(quote! {
                    stringify!(#ident) => return self.#ident.set_field(rest, value),
                });
            }
            continue;
        }

        // ── a leaf ──
        let f = quote! { crate::bits::Field::<#ty, #lo, #hi> };
        decode.push(quote! { #ident: #f::get(&raw)? });
        encode.push(quote! { #f::set(&mut raw, p.#ident); });

        let path = ident.to_string();
        layout.push(quote! {
            crate::layout::LayoutField {
                path: #path,
                ty: #ty_str,
                lo: #lo,
                hi: #hi,
                nested: ::core::option::Option::None,
            }
        });

        if !public {
            continue;
        }
        let placement = format!("{lo}..={hi}");
        let width = hi - lo + 1;

        values.push(quote! {
            out.push(crate::fields::FieldValue {
                name: #path.to_string(),
                placement: #placement,
                raw: crate::bits::Field::<u64, #lo, #hi>::read(&self.raw),
                bits: <#ty as crate::bits::Packed>::to_bits(&self.#ident),
                value: ::std::format!("{:?}", &self.#ident),
            });
        });
        // What the type cannot know, taken from the field's own name and applied to the
        // kinds that have a use for it — every other kind ignores the refinement.
        let mut control = quote! { <#ty as crate::bits::Packed>::CONTROL };
        if let Some(parent) = morphed_parent(&path, &registered) {
            control = quote! { #control.morphing(#parent) };
        }
        if let Some(rank) = trailing_ordinal(&path) {
            control = quote! { #control.ranked(#rank) };
        }

        specs.push(quote! {
            out.push(crate::fields::FieldSpec {
                name: #path.to_string(),
                placement: #placement,
                width: #width,
                legal: || crate::fields::legal_values::<#ty>(#width),
                control: #control,
            });
        });
        // The parse is the type's, so a value it cannot hold fails here rather
        // than being clamped into the slot.
        setters.push(quote! {
            #path => {
                self.#ident = crate::fields::parse_field::<#ty>(#width, value)
                    .map_err(|e| e.at(#path))?;
                return Ok(());
            }
        });
    }

    let gaps = unclaimed(
        &claimed.iter().map(|&(l, h, _)| (l, h)).collect::<Vec<_>>(),
        span_bits,
    );
    let gap_doc = if gaps.is_empty() {
        format!("Every one of the body's {span_bits} bits is named.")
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

    rows.extend(gaps.iter().map(|&(lo, hi)| MapRow {
        lo,
        hi,
        field: None,
    }));
    let map = map_table(rows);

    Ok(quote! {
        #(#attrs)*
        #[doc = ""]
        #[doc = #gap_doc]
        #[doc = ""]
        #[doc = "The body's map, in offset order:"]
        #[doc = ""]
        #(#[doc = #map])*
        #vis struct #name {
            /// The bytes this body was decoded from, so bits no field claims survive a
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

        impl crate::cbin::Body for #name {
            const LEN: ::core::option::Option<u64> =
                ::core::option::Option::Some(#bytes as u64);

            fn read<R: ::std::io::Read + ::std::io::Seek>(
                r: &mut crate::cbin::BodyReader<'_, R>,
                _: &crate::cbin::Header,
            ) -> ::core::result::Result<Self, crate::error::Error> {
                let mut raw = [0u8; #bytes];
                ::std::io::Read::read_exact(r, &mut raw)?;
                Ok(::core::convert::TryFrom::try_from(raw)?)
            }

            fn write<W: ::std::io::Write + ::std::io::Seek>(
                &self,
                w: &mut crate::cbin::BodyWriter<'_, W>,
            ) -> ::core::result::Result<(), crate::error::Error> {
                ::std::io::Write::write_all(w, &<[u8; #bytes]>::from(self))?;
                Ok(())
            }
        }

        impl crate::layout::BodyLayout for #name {
            fn layout() -> &'static [crate::layout::LayoutField] {
                const FIELDS: &[crate::layout::LayoutField] = &[#(#layout,)*];
                FIELDS
            }
        }

        impl #name {
            /// Every registered field's current value, under its full path, in
            /// declaration order — nested bodies inline where their field sits.
            /// Describes the same fields as [`Self::field_specs`], so callers may
            /// zip the two positionally.
            pub fn field_values(&self) -> ::std::vec::Vec<crate::fields::FieldValue> {
                let mut out = ::std::vec::Vec::new();
                #(#values)*
                out
            }

            pub fn field_specs() -> ::std::vec::Vec<crate::fields::FieldSpec> {
                let mut out = ::std::vec::Vec::new();
                #(#specs)*
                out
            }

            /// Every settable field, described under its full path.
            pub fn fields(&self) -> ::std::vec::Vec<crate::fields::Field> {
                Self::field_specs()
                    .into_iter()
                    .zip(self.field_values())
                    .map(|(spec, value)| crate::fields::Field {
                        path: spec.name.clone(),
                        value: crate::fields::settable_form(spec.width, &value.value, value.bits),
                        display: value.value,
                        spec,
                    })
                    .collect()
            }

            /// Set one field by its full path.
            pub fn set_field(
                &mut self,
                path: &str,
                value: &str,
            ) -> ::core::result::Result<(), crate::fields::FieldError> {
                match path {
                    #(#setters)*
                    _ => {}
                }
                if let ::core::option::Option::Some((head, rest)) = path.split_once('.') {
                    match head {
                        #(#routes)*
                        _ => {}
                    }
                }
                Err(crate::fields::FieldError::UnknownField {
                    panel: stringify!(#name),
                    name: path.to_string(),
                })
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
    fn a_hex_span_collapses_whole_bytes() {
        assert_eq!(hex_span(0, 15), "`0x00..0x02`");
        assert_eq!(hex_span(24, 31), "`0x03`");
        assert_eq!(hex_span(24, 27), "`0x03` bits 7..4");
        assert_eq!(hex_span(23, 23), "`0x02` bit 0");
        assert_eq!(hex_span(61, 67), "`0x07` bits 2..0; `0x08` bits 7..4");
    }

    #[test]
    fn the_map_tiles_the_body_in_offset_order() {
        let row = |lo, hi, field: Option<(&str, &str)>| MapRow {
            lo,
            hi,
            field: field.map(|(n, t)| (n.to_string(), t.to_string())),
        };
        assert_eq!(
            map_table(vec![
                row(16, 18, Some(("level", "RangedU8<3>"))),
                row(0, 15, Some(("word", "u16"))),
                row(19, 23, None),
            ]),
            [
                "| bytes | bits | field | type |",
                "|---|---|---|---|",
                "| `0x00..0x02` | `0..=15` | `word` | [`u16`] |",
                "| `0x02` bits 7..5 | `16..=18` | `level` | `RangedU8<3>` |",
                "| `0x02` bits 4..0 | `19..=23` | — | *unclaimed* |",
            ],
        );
    }

    /// The binding is by name, and only to a parameter the body actually registers.
    #[test]
    fn a_morph_slot_binds_to_the_parameter_beside_it() {
        let registered = ["organ_a_volume", "drawbar_1", "delay_tempo"];
        assert_eq!(
            morphed_parent("organ_a_volume_ctrl_pedal", &registered),
            Some("organ_a_volume"),
        );
        assert_eq!(
            morphed_parent("drawbar_1_wheel", &registered),
            Some("drawbar_1")
        );
        // No such parameter in this body, and no suffix at all.
        assert_eq!(morphed_parent("piano_a_volume_wheel", &registered), None);
        assert_eq!(morphed_parent("delay_tempo", &registered), None);
        // ⚠️ A mangled name is not a morph slot: the Stage 2 has a
        // `…_wheel_o_delay_on` whose suffix is `_on`.
        assert_eq!(
            morphed_parent("delay_tempo_wheel_o_delay_on", &registered),
            None
        );
    }

    #[test]
    fn an_ordinal_is_read_off_the_end_of_a_name() {
        assert_eq!(trailing_ordinal("drawbar_1"), Some(1));
        assert_eq!(trailing_ordinal("organ_a_drawbar_9"), Some(9));
        assert_eq!(trailing_ordinal("b3_preset1_drawbars"), None);
        assert_eq!(trailing_ordinal("b3_bass_bar1"), None);
        assert_eq!(trailing_ordinal("kb_zones_1_2_split_point"), None);
        assert_eq!(trailing_ordinal("gain"), None);
    }

    #[test]
    fn unclaimed_finds_the_holes_and_both_ends() {
        assert_eq!(unclaimed(&[(0, 2), (5, 9)], 16), vec![(3, 4), (10, 15)]);
        assert_eq!(unclaimed(&[(0, 7)], 8), vec![]);
        assert_eq!(unclaimed(&[(4, 7)], 8), vec![(0, 3)]);
    }
}
