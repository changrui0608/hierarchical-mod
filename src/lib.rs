#![feature(proc_macro_span)]

use std::collections::BTreeMap;

use syn::parse_macro_input;

struct Arg {
    path: syn::LitStr,
}

impl syn::parse::Parse for Arg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Arg {
            path: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn path_mod(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Arg);
    let rel_path = std::path::PathBuf::from(&input.path.value());

    _mod(&rel_path)
}

#[proc_macro]
pub fn auto_mod(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = proc_macro::Span::call_site();

    assert!(
        span.source_file().is_real(),
        "using hierarchical-mod via macros is not supported",
    );

    let rel_path = match span.source_file().path().parent() {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from("leetcode-algorithms"),
    };

    _mod(&rel_path)
}

fn _mod(rel_path: &std::path::Path) -> proc_macro::TokenStream {
    let path = match std::env::var_os("CARGO_MANIFEST_DIR") {
        Some(manifest_dir) => std::path::PathBuf::from(manifest_dir).join(rel_path),
        None => std::path::PathBuf::from(rel_path),
    };

    let token_stream = gen_stream(&path).unwrap();
    proc_macro::TokenStream::from(token_stream)
}

fn gen_stream(path: &std::path::Path) -> Result<proc_macro2::TokenStream> {
    let mut srcs = Vec::new();
    let mut dirs = BTreeMap::new();

    for entry in path.read_dir()? {
        let entry = entry?;

        let path = entry.path();
        let file_name = entry.file_name();

        match entry.file_type()? {
            file_type if file_type.is_file() => {
                if path.extension() != Some(std::ffi::OsStr::new("rs")) {
                    continue;
                }

                // TODO: what about rust 2018 style mods? e.g.
                //
                // |- foo
                // |  |- foo1.rs
                // |  |- foo2.rs
                // |- foo.rs
                if file_name == "mod.rs" {
                    continue;
                }

                // TODO: hard-coded filename, will try to read from cargo env or something
                if file_name == "lib.rs" || file_name == "main.rs" {
                    continue;
                }

                srcs.push(file_name.into_string()?);
            }
            file_type if file_type.is_dir() => {
                let sub_dirs = gen_stream(path.as_path())?;
                if !sub_dirs.is_empty() {
                    dirs.insert(file_name.into_string()?, sub_dirs); // XXX: from `OsString` to `Error` is not good.
                }
            }
            file_type => panic!("unknown file type: {:?}", file_type),
        }
    }

    let srcs = srcs;
    let src_names = srcs.iter();
    let src_mod_names = srcs
        .iter()
        .map(String::clone)
        .map(handle_mod_name_with_dash)
        .map(handle_mod_name_with_digit_prefix)
        .map(strip_rs_suffix)
        .map(|x| proc_macro2::Ident::new(&x, proc_macro2::Span::call_site()));

    let dirs = dirs;
    let dir_names = dirs.keys();

    let dir_mod_names = dirs
        .keys()
        .map(String::clone)
        .map(handle_mod_name_with_dash)
        .map(handle_mod_name_with_digit_prefix)
        .map(|x| proc_macro2::Ident::new(&x, proc_macro2::Span::call_site()));

    let dir_streams = dirs.values();

    Ok(quote::quote! {
        #(
            #[path = #dir_names]
            pub mod #dir_mod_names {
                #dir_streams
            }
        )*
        #(
            #[path = #src_names]
            pub mod #src_mod_names;
        )*
    })
}

fn handle_mod_name_with_dash(s: String) -> String {
    s.replace('-', "_")
}

fn handle_mod_name_with_digit_prefix(file_name: String) -> String {
    if file_name.chars().next().unwrap().is_ascii_digit() {
        format!("_{file_name}")
    } else {
        file_name
    }
}

fn strip_rs_suffix(file_name: String) -> String {
    file_name
        .strip_suffix(".rs")
        .unwrap_or(&file_name)
        .to_string()
}

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Utf8(std::ffi::OsString),
}

type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(e) => {
                write!(f, "IO Error: {}", e)
            }
            Error::Utf8(_) => {
                write!(f, "Unicode Error")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::ffi::OsString> for Error {
    fn from(value: std::ffi::OsString) -> Self {
        Self::Utf8(value)
    }
}
