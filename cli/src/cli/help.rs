/// Print version information.
pub fn print_version() {
    println!("rick v{}", env!("CARGO_PKG_VERSION"));
}

/// Print help/usage information.
pub fn print_help() {
    println!("\x1b[36mRick - Multi-agent AI Orchestration CLI\x1b[0m");
    println!();
    println!("Usage: rick <command> [options]");
    println!();
    println!("Commands:");
    println!("  setup [options]     Onboarding wizard: install skill, persona, permissions, auto-update");
    println!("  add <url> [-n name] Clone an existing Universe and compile its agents");
    println!("  init                Initialize a new Rick universe");
    println!("  compile             Compile agents to Claude Code sub-agents");
    println!("  check               Verify all agent dependencies (MCPs, skills)");
    println!("  push                Commit and push Universe changes, then recompile");
    println!("  invite [users...]   Invite GitHub users as collaborators + show install link");
    println!("  list agents         List all agents in the universe");
    println!("  list workflows      List all workflows in the universe");
    println!("  run <workflow> [-f] Start a workflow (--force to skip dep checks)");
    println!("  next                Continue to next workflow step");
    println!("  status              Show active workflow status");
    println!("  help                Show this help message");
    println!();
    println!("Setup options:");
    println!("  --universe, -u <url>  Clone and compile a Universe during setup");
    println!("  --install-deps        Auto-install MCP servers required by agents");
    println!();
    println!("Options:");
    println!("  -h, --help          Show help");
    println!("  -v, --version       Show version");
}
