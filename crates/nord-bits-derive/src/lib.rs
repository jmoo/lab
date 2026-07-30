//! `#[bitpanel]` — declare a bit-packed panel as an ordinary Rust struct.
//!
//! ```ignore
//! #[bitpanel(settings: u16, settings2: u8)]
//! #[derive(Default)]
//! pub struct CenterPanel {
//!     #[bits(settings, 15..=13)]
//!     pub left_part: Instrument,
//!     #[bits(settings3, 20..=14)]
//!     pub gain: u8,
//!     /// A value split across two words.
//!     #[bits(settings, 2..=0, settings2, 31..=28)]
//!     pub equalizer_freq_gain: u8,
//! }
//! ```
//!
//! Generates the packed `<Name>Words` struct, both directions of the conversion, and a
//! `Debug` over the decoded fields. Bits no field claims are preserved.
//!
//! A field spelled as a raw `u8`/`u16`/`u32`/`u64` may be wider than its slot, so it is
//! written with a checked call and encoding is fallible; any other type carries its own
//! range, so the write is unchecked and the fit is proven at compile time. When no field
//! can overflow, the generated encode is `From` rather than `TryFrom`.
//!
//! Only usable inside `nord-format`: generated code names `crate::bits` and `crate::error`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprRange, Field, Ident, ItemStruct, Lit, RangeLimits, Token, Type};

/// One `word[HI..=LO]` placement.
struct Placement {
    word: Ident,
    hi: u32,
    lo: u32,
}

/// One placement, or two for a value split across words.
struct Bits(Vec<Placement>);

impl Parse for Bits {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut places = Vec::new();

        while !input.is_empty() {
            let word: Ident = input.parse()?;
            input.parse::<Token![,]>()?;
            let range: ExprRange = input.parse()?;

            if !matches!(range.limits, RangeLimits::Closed(_)) {
                return Err(syn::Error::new_spanned(
                    &range,
                    "bit ranges are inclusive: write `15..=13`, not `15..13`",
                ));
            }

            let hi = literal(range.start.as_deref(), &range, "high bit")?;
            let lo = literal(range.end.as_deref(), &range, "low bit")?;

            if hi < lo {
                return Err(syn::Error::new_spanned(
                    &range,
                    format!("bit range runs backwards: `{hi}..={lo}` (write the high bit first)"),
                ));
            }

            places.push(Placement { word, hi, lo });

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        match places.len() {
            1 | 2 => Ok(Bits(places)),
            n => Err(input.error(format!(
                "expected one `word, HI..=LO` placement, or two for a value split across \
                 words; found {n}"
            ))),
        }
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

/// Width of a raw unsigned integer type, if that is what this is.
fn raw_integer_bits(ty: &Type) -> Option<u32> {
    let Type::Path(p) = ty else { return None };
    if p.qself.is_some() {
        return None;
    }
    match p.path.get_ident()?.to_string().as_str() {
        "u8" => Some(8),
        "u16" => Some(16),
        "u32" => Some(32),
        "u64" => Some(64),
        _ => None,
    }
}

/// Whether writing this field can overflow its slot: only a raw integer wider than the
/// slot can.
fn may_overflow(ty: &Type, width: u32) -> bool {
    raw_integer_bits(ty).is_some_and(|bits| bits > width)
}

/// Whether every bit pattern is a valid value.
fn decodes_totally(ty: &Type) -> bool {
    raw_integer_bits(ty).is_some()
        || matches!(ty, Type::Path(p) if p.qself.is_none()
            && p.path.get_ident().is_some_and(|i| i == "bool"))
}

#[proc_macro_attribute]
pub fn bitpanel(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let parse_words = |input: ParseStream| {
        Punctuated::<Field, Token![,]>::parse_terminated_with(input, Field::parse_named)
    };
    let words = parse_words.parse2(attr.clone()).map_err(|_| {
        syn::Error::new(
            attr.span(),
            "expected the packed words, as `name: type` pairs — e.g. \
             `#[bitpanel(settings: u16, settings2: u8)]`",
        )
    })?;

    if words.is_empty() {
        return Err(syn::Error::new(
            attr.span(),
            "a panel needs at least one backing word",
        ));
    }

    let panel: ItemStruct = syn::parse2(item)?;
    let name = &panel.ident;
    let vis = &panel.vis;
    let attrs = &panel.attrs;
    let words_name = format_ident!("{}Words", name);

    let syn::Fields::Named(named) = &panel.fields else {
        return Err(syn::Error::new_spanned(
            &panel,
            "a panel must be a struct with named fields",
        ));
    };

    // Every Nord panel word is big-endian.
    let word_defs = words.iter().map(|w| {
        let ident = &w.ident;
        let ty = &w.ty;
        quote! { #[brw(big)] pub(crate) #ident: #ty }
    });

    let mut any_fallible = false;

    let mut decode = Vec::new();
    let mut encode = Vec::new();
    let mut debug = Vec::new();
    let mut fields = Vec::new();

    for field in &named.named {
        let ident = field.ident.as_ref().expect("named fields");
        let ty = &field.ty;

        let bits = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident("bits"))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    field,
                    "every field of a panel needs a `#[bits(word, HI..=LO)]` placement",
                )
            })?;
        let Bits(places) = bits.parse_args()?;

        // Keep everything but the placement, which has served its purpose.
        let kept: Vec<_> = field
            .attrs
            .iter()
            .filter(|a| !a.path().is_ident("bits"))
            .collect();
        let field_vis = &field.vis;
        fields.push(quote! { #(#kept)* #field_vis #ident: #ty });

        let total = decodes_totally(ty);
        let width: u32 = places.iter().map(|p| p.hi - p.lo + 1).sum();
        let fallible = may_overflow(ty, width);
        any_fallible |= fallible;

        match places.as_slice() {
            [p] => {
                let (word, hi, lo) = (&p.word, p.hi, p.lo);
                let f = quote! { crate::bits::Field::<#ty, #hi, #lo> };
                decode.push(if total {
                    quote! { #ident: #f::read(w.#word) }
                } else {
                    quote! { #ident: #f::get(w.#word)? }
                });
                encode.push(if fallible {
                    quote! { #f::checked_set(&mut w.#word, p.#ident)?; }
                } else {
                    quote! { #f::set(&mut w.#word, p.#ident); }
                });
            }
            [high, low] => {
                let (hw, hh, hl) = (&high.word, high.hi, high.lo);
                let (lw, lh, ll) = (&low.word, low.hi, low.lo);
                let s = quote! {
                    crate::bits::Straddle::<
                        #ty,
                        crate::bits::Field<#ty, #hh, #hl>,
                        crate::bits::Field<#ty, #lh, #ll>,
                    >
                };
                decode.push(if total {
                    quote! { #ident: #s::read(w.#hw, w.#lw) }
                } else {
                    quote! { #ident: #s::get(w.#hw, w.#lw)? }
                });
                encode.push(if fallible {
                    quote! { #s::checked_set(&mut w.#hw, &mut w.#lw, p.#ident)?; }
                } else {
                    quote! { #s::set(&mut w.#hw, &mut w.#lw, p.#ident); }
                });
            }
            _ => unreachable!("Bits::parse rejects any other count"),
        }

        debug.push(quote! { .field(stringify!(#ident), &self.#ident) });
    }

    // `From` when every field provably fits, `TryFrom` otherwise — so retyping a field
    // to a raw integer breaks the caller's `#[bw(map)]` rather than silently going
    // fallible.
    let encode_impl = if any_fallible {
        quote! {
            impl ::core::convert::TryFrom<&#name> for #words_name {
                type Error = crate::bits::FieldOverflow;

                fn try_from(p: &#name) -> ::core::result::Result<Self, Self::Error> {
                    let mut w = p.words;
                    #(#encode)*
                    Ok(w)
                }
            }
        }
    } else {
        quote! {
            impl ::core::convert::From<&#name> for #words_name {
                fn from(p: &#name) -> Self {
                    let mut w = p.words;
                    #(#encode)*
                    w
                }
            }
        }
    };

    Ok(quote! {
        /// The panel's packed words, as they sit on disk.
        #[::binrw::binrw]
        #[derive(Clone, Copy, Debug, Default)]
        #vis struct #words_name {
            #(#word_defs,)*
        }

        #(#attrs)*
        #vis struct #name {
            /// The words this panel was decoded from, so bits no field claims survive a
            /// re-encode. Named fields take precedence on write.
            words: #words_name,
            #(#fields,)*
        }

        impl ::core::convert::TryFrom<#words_name> for #name {
            type Error = crate::error::ParseError;

            fn try_from(w: #words_name) -> ::core::result::Result<Self, Self::Error> {
                Ok(#name { words: w, #(#decode,)* })
            }
        }

        #encode_impl

        /// The decoded values; the backing words are not printed.
        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #(#debug)*
                    .finish()
            }
        }
    })
}
