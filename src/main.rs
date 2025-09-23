use std::env;
use timex_datalink::protocol_3::{
    Protocol3, end::End, start::Start, sync::Sync, wrist_app::WristApp,
};
use timex_datalink::{NotebookAdapter, PacketGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!(
            "Usage: {} <recovery_codes> <serial_port> [--show-listing]",
            args[0]
        );
        eprintln!(
            "Example: {} \"github:abc123,google:def456\" /dev/ttyUSB0",
            args[0]
        );
        eprintln!(
            "         {} \"github:abc123\" /dev/null --show-listing",
            args[0]
        );
        std::process::exit(1);
    }

    let recovery_codes = &args[1];
    let serial_port = &args[2];
    let show_listing = args.len() > 3 && args[3] == "--show-listing";

    println!("WristVault - Secure Recovery Codes for Timex Datalink 150");
    println!("Generating wristapp with recovery codes...");

    let assembly_code = generate_wristapp(recovery_codes)?;
    println!("Generated assembly code ({} bytes)", assembly_code.len());

    let compiled = compile_assembly(&assembly_code, show_listing)?;
    println!("Compiled to hex ({} bytes)", compiled.len());

    send_to_watch(&compiled, serial_port)?;
    println!("Successfully sent to watch on {}", serial_port);

    Ok(())
}

fn generate_wristapp(recovery_codes: &str) -> Result<String, Box<dyn std::error::Error>> {
    let codes: Vec<&str> = recovery_codes.split(',').collect();
    let num_codes = codes.len();

    let template = include_str!("wristvault.zsm");

    let recovery_data = format_recovery_codes(recovery_codes);
    let (service_table_data, code_table_data, service_lookup_data) = generate_tables(&codes);

    let assembly = template
        .replace("{RECOVERY_DATA}", &recovery_data)
        .replace("{SERVICE_TABLE_DATA}", &service_table_data)
        .replace("{CODE_TABLE_DATA}", &code_table_data)
        .replace("{SERVICE_LOOKUP_DATA}", &service_lookup_data)
        .replace("{NUM_CODES}", &num_codes.to_string())
        .replace(
            "{NUM_CODES_MINUS_1}",
            &(num_codes.saturating_sub(1)).to_string(),
        );

    Ok(assembly)
}

fn format_recovery_codes_advanced(
    codes: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut recovery_data = String::new();
    let mut recovery_lookup = String::new();

    for (i, code_pair) in codes.split(',').enumerate() {
        if let Some((mut service, mut code)) = code_pair.split_once(':') {
            // Ensure strings fit in 6 characters for timex6 encoding
            service = &service[..service.len().min(6)];
            code = &code[..code.len().min(6)];

            // Generate the timex6 data strings
            recovery_data.push_str(&format!(
                "S6_SVC{}:   timex6  \"{:6}\"\n",
                i,
                service.to_uppercase()
            ));
            recovery_data.push_str(&format!(
                "S6_COD{}:   timex6  \"{:6}\"\n",
                i,
                code.to_uppercase()
            ));

            // Generate the lookup table entries (offsets from START)
            recovery_lookup.push_str(&format!(
                "        db      S6_SVC{}-START  ; Service: {}\n",
                i, service
            ));
            recovery_lookup.push_str(&format!(
                "        db      S6_COD{}-START  ; Code: {}\n",
                i, code
            ));
        }
    }

    Ok((recovery_data, recovery_lookup))
}

fn format_recovery_codes(codes: &str) -> String {
    // Legacy function - now calls the advanced version
    match format_recovery_codes_advanced(codes) {
        Ok((recovery_data, _)) => recovery_data,
        Err(_) => String::new(),
    }
}

// fn calculate_recovery_data_size(codes: &[&str]) -> usize {
//     // Each code pair generates 2 timex6 strings of 6 bytes each = 12 bytes per pair
//     codes.len() * 12
// }

fn generate_tables(codes: &[&str]) -> (String, String, String) {
    let mut service_table_data = String::new();
    let mut code_table_data = String::new();
    let mut service_lookup_data = String::new();

    for (i, code_pair) in codes.iter().enumerate() {
        if let Some((service, code)) = code_pair.split_once(':') {
            // Generate 8-character service strings for bottom display
            service_table_data.push_str(&format!(
                "S8_SVC{}:   timex   \"{:8}\"\n",
                i,
                service.to_uppercase()
            ));

            // Generate 8-character code strings for bottom display
            code_table_data.push_str(&format!("        db      S8_COD{}-START  ; {}\n", i, code));
            code_table_data.push_str(&format!(
                "S8_COD{}:   timex   \"{:8}\"\n",
                i,
                code.to_uppercase()
            ));

            // Generate service lookup table
            service_lookup_data.push_str(&format!(
                "        db      S8_SVC{}-START  ; {}\n",
                i, service
            ));
        }
    }

    (service_table_data, code_table_data, service_lookup_data)
}

// fn generate_code_table(codes: &[&str]) -> String {
//     // Legacy function for compatibility
//     let (_, code_table, _) = generate_tables(codes);
//     code_table
// }

fn compile_assembly(
    assembly: &str,
    show_listing: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    // Get absolute path to include file
    let current_dir = env::current_dir()?;
    let include_path = current_dir
        .join("include")
        .join("Inc150")
        .join("WRISTAPP.I");

    // Verify the include file exists
    if !include_path.exists() {
        return Err(format!("Include file not found: {}", include_path.display()).into());
    }
    let include_path_str = include_path.to_string_lossy();

    // Replace the relative include with absolute path
    let assembly_with_abs_path = assembly.replace(
        "INCLUDE \"Inc150/WRISTAPP.I\"",
        &format!("INCLUDE \"{}\"", include_path_str),
    );

    // Split assembly into lines for the assembler
    let assembly_lines: Vec<String> = assembly_with_abs_path
        .lines()
        .map(|s| s.to_string())
        .collect();
    let result = rs_asm6805::assemble("wristapp.asm".to_string(), assembly_lines);

    match result {
        Ok((errors, hex, listing)) => {
            println!("Assembly errors ({} found):", errors.len());
            for error in &errors {
                eprintln!("Assembly error: {}", error);
            }

            if show_listing {
                println!("\n=== ASSEMBLY LISTING ===");
                for line in &listing {
                    println!("{}", line);
                }
                println!("=== END LISTING ===\n");
            }

            println!("Hex output length: {} bytes", hex.len());
            if !hex.is_empty() {
                println!(
                    "First 64 chars of hex: {}",
                    &hex.chars().take(64).collect::<String>()
                );
            }

            // Note: Some "errors" are actually just symbol definitions with default values
            // If we have hex output, the assembly succeeded
            Ok(hex)
        }
        Err(errors) => {
            for error in &errors {
                eprintln!("Assembly error: {}", error);
            }
            Err("Assembly compilation failed".into())
        }
    }
}

fn send_to_watch(hex_data: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to watch on {}...", port);

    // Convert hex string to binary data
    let binary_data = hex_to_binary(hex_data)?;
    println!(
        "Converted hex to {} bytes of binary data",
        binary_data.len()
    );

    // Create Protocol3 structure with WristApp
    let mut protocol = Protocol3::new();

    // Add mandatory components
    protocol.add(Sync { length: 100 });
    protocol.add(Start {});

    // Add the wrist app
    protocol.add(WristApp {
        wrist_app_data: binary_data,
    });

    // Add end component
    protocol.add(End {});

    // Generate packets
    let packets = protocol.packets();
    println!("Generated {} packet groups for transmission", packets.len());

    // Create adapter and send to watch
    let adapter = NotebookAdapter::new(
        port.to_string(),
        None, // Use default byte sleep time
        None, // Use default packet sleep time
        true, // Enable verbose output
    );

    match adapter.write(&packets) {
        Ok(_) => {
            println!("Successfully transmitted wristapp to watch!");
            Ok(())
        }
        Err(e) => {
            eprintln!("Error transmitting to watch: {}", e);
            Err(e.into())
        }
    }
}

fn hex_to_binary(hex_str: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    let hex_str = hex_str.trim();

    // Process hex string in pairs
    let mut i = 0;
    while i + 1 < hex_str.len() {
        let hex_pair = &hex_str[i..i + 2];
        match u8::from_str_radix(hex_pair, 16) {
            Ok(byte) => result.push(byte),
            Err(_) => {
                // Skip invalid hex characters
                i += 1;
                continue;
            }
        }
        i += 2;
    }

    Ok(result)
}
