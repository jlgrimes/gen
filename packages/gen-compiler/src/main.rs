use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: gen <input.gen> [output.xml]");
        eprintln!("       gen --no-validate <input.gen> [output.xml]");
        process::exit(1);
    }

    let mut no_validate = false;
    let mut input_path = &args[1];
    let mut output_path: Option<&String> = args.get(2);

    // Parse flags
    if args[1] == "--no-validate" {
        no_validate = true;
        if args.len() < 3 {
            eprintln!("Usage: gen --no-validate <input.gen> [output.xml]");
            process::exit(1);
        }
        input_path = &args[2];
        output_path = args.get(3);
    }

    // Read input file
    let source = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    // Compile
    let result = if no_validate {
        gen::compile_unchecked(&source)
    } else {
        gen::compile(&source)
    };

    let xml = match result {
        Ok(xml) => xml,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    // Output
    match output_path {
        Some(path) => {
            if let Err(e) = fs::write(path, &xml) {
                eprintln!("Error writing to '{}': {}", path, e);
                process::exit(1);
            }
            eprintln!("Wrote MusicXML to {}", path);
        }
        None => {
            println!("{}", xml);
        }
    }
}
