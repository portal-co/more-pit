use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use pit_core::parse_interface;

fn compute_rid(source: &str, label: &str, multiple: bool) -> Result<(), String> {
    let (_rest, iface) = parse_interface(source)
        .map_err(|e| format!("{label}: parse error: {e}"))?;
    let rid = iface.rid_str();
    if multiple {
        println!("{rid}  {label}");
    } else {
        println!("{rid}");
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        let mut src = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut src) {
            eprintln!("pit-rid: error reading stdin: {e}");
            process::exit(1);
        }
        if let Err(e) = compute_rid(&src, "<stdin>", false) {
            eprintln!("pit-rid: {e}");
            process::exit(1);
        }
    } else {
        let multiple = args.len() > 1;
        let mut had_error = false;
        for path in &args {
            let src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("pit-rid: {path}: {e}");
                    had_error = true;
                    continue;
                }
            };
            if let Err(e) = compute_rid(&src, path, multiple) {
                eprintln!("pit-rid: {e}");
                had_error = true;
            }
        }
        if had_error {
            process::exit(1);
        }
    }
}
