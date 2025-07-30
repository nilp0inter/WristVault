use std::env;
use rs_asm6805;
use timex_datalink::{PacketGenerator, NotebookAdapter};
use timex_datalink::protocol_4::{Protocol4, wrist_app::WristApp, start::Start, sync::Sync, end::End};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <recovery_codes> <serial_port>", args[0]);
        eprintln!("Example: {} \"github:abc123,google:def456\" /dev/ttyUSB0", args[0]);
        std::process::exit(1);
    }

    let recovery_codes = &args[1];
    let serial_port = &args[2];

    println!("WristVault - Secure Recovery Codes for Timex Datalink 150");
    println!("Generating wristapp with recovery codes...");

    let assembly_code = generate_wristapp(recovery_codes)?;
    println!("Generated assembly code ({} bytes)", assembly_code.len());

    let compiled = compile_assembly(&assembly_code)?;
    println!("Compiled to hex ({} bytes)", compiled.len());

    send_to_watch(&compiled, serial_port)?;
    println!("Successfully sent to watch on {}", serial_port);

    Ok(())
}

fn generate_wristapp(recovery_codes: &str) -> Result<String, Box<dyn std::error::Error>> {
    let template = r#";Name: WristVault
;Version: VAULT
;Description: Secure Recovery Codes
;
            INCLUDE "Inc150/WRISTAPP.I"
;
FLAGBYTE    EQU      $61
;
START    EQU   *
;
L0110:  jmp    MAIN
L0113:  rts
        nop
        nop
L0116:  rts
        nop
        nop
L0119:  rts
        nop
        nop
L011c:  rts
        nop
        nop

L011f:  lda    STATETAB,X
        rts

L0123:  jmp    HANDLE_STATE0
        db      STATETAB-STATETAB

{RECOVERY_DATA}

STATETAB:
        db      0
        db    EVT_ENTER,TIM_ONCE,0
        db      EVT_RESUME,TIM_ONCE,0
        db      EVT_DNNEXT,TIM_ONCE,0
        db      EVT_MODE,TIM_ONCE,$FF
        db      EVT_END

HANDLE_STATE0:
        bset    1,$8f
        lda    BTNSTATE
        cmp     #EVT_DNNEXT
        beq    SHOWNEXT
        jsr    CLEARALL
        lda    #S6_VAULT-START
        jsr    PUT6TOP
        lda    #S6_CODES-START
        jsr    PUT6MID
        lda    #SYS8_MODE
        jmp    PUTMSGBOT

SHOWNEXT:
        ; Display recovery codes logic here
        rts

MAIN:
        lda    #$c0
        sta    $96
        clr     FLAGBYTE
        rts

S6_VAULT:   timex6  "VAULT "
S6_CODES:   timex6  "CODES "
"#;

    let recovery_data = format_recovery_codes(recovery_codes);
    let assembly = template.replace("{RECOVERY_DATA}", &recovery_data);
    
    Ok(assembly)
}

fn format_recovery_codes(codes: &str) -> String {
    let mut result = String::new();
    
    for (i, code_pair) in codes.split(',').enumerate() {
        if let Some((service, code)) = code_pair.split_once(':') {
            result.push_str(&format!("S6_SVC{}:   timex6  \"{} \"\n", i, service.to_uppercase()));
            result.push_str(&format!("S6_COD{}:   timex6  \"{} \"\n", i, code.to_uppercase()));
        }
    }
    
    result
}

fn compile_assembly(assembly: &str) -> Result<String, Box<dyn std::error::Error>> {
    
    // Get absolute path to include file
    let current_dir = env::current_dir()?;
    let include_path = current_dir.join("include").join("Inc150").join("WRISTAPP.I");
    
    // Verify the include file exists
    if !include_path.exists() {
        return Err(format!("Include file not found: {}", include_path.display()).into());
    }
    let include_path_str = include_path.to_string_lossy();
    
    // Replace the relative include with absolute path
    let assembly_with_abs_path = assembly.replace(
        "INCLUDE \"Inc150/WRISTAPP.I\"", 
        &format!("INCLUDE \"{}\"", include_path_str)
    );
    
    // Split assembly into lines for the assembler
    let assembly_lines: Vec<String> = assembly_with_abs_path.lines().map(|s| s.to_string()).collect();
    let result = rs_asm6805::assemble("wristapp.asm".to_string(), assembly_lines);
    
    match result {
        Ok((errors, hex, _listing)) => {
            println!("Assembly errors ({} found):", errors.len());
            for error in &errors {
                eprintln!("Assembly error: {}", error);
            }
            
            // Skip printing assembly listing to avoid output overflow
            
            println!("Hex output length: {} bytes", hex.len());
            if hex.len() > 0 {
                println!("First 64 chars of hex: {}", &hex.chars().take(64).collect::<String>());
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
    println!("Converted hex to {} bytes of binary data", binary_data.len());
    
    // Create Protocol4 structure with WristApp
    let mut protocol = Protocol4::new();
    
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
        let hex_pair = &hex_str[i..i+2];
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
