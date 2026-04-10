//! Thaleia MCP Server - Standalone binary entry point
//!
//! This binary runs the MCP server with rmcp SDK v1.3.

use std::process::exit;

fn main() {
    // Initialize logging first
    thaleia_core::init_logging();

    let args: Vec<String> = std::env::args().collect();

    let mut mode = "standard";
    let mut debug = false;

    for i in 1..args.len() {
        match args[i].as_str() {
            "--mode" => {
                if i + 1 < args.len() {
                    mode = &args[i + 1];
                }
            }
            "--debug" => {
                debug = true;
            }
            "-h" | "--help" => {
                println!("Thaleia MCP Server v0.1.0 (rmcp v1.3)");
                println!();
                println!("Usage: thaleia-mcp [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --mode <mode>   Session mode: ephemeral, standard, longterm");
                println!("  --debug         Enable debug output (verbose)");
                println!("  -h, --help      Show this help");
                println!();
                println!("Environment:");
                println!("  RUST_LOG=debug  Enable tracing debug level");
                println!("  THALEIA_DEBUG=1 Enable verbose debug prints");
                exit(0);
            }
            _ => {}
        }
    }

    // Enable debug mode if flag provided
    if debug {
        thaleia_core::set_debug(true);
        eprintln!("Starting Thaleia MCP server (mode: {}, debug: ON)", mode);
    }

    // Set mode - unsafe in multi-threaded but OK in single-threaded main
    unsafe {
        std::env::set_var("THALEIA_MODE", mode);
    }

    // Run the server
    match thaleia_mcp::rmcp_server::run_server() {
        Ok(()) => exit(0),
        Err(e) => {
            eprintln!("MCP server error: {}", e);
            exit(1);
        }
    }
}
