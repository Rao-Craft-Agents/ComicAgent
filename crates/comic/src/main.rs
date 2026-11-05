//! `comic`: the runner a host calls over the wire, and a few commands for a person at a terminal.
//!
//! ```text
//! comic                        answer one call on stdin and stdout (granite-agent/1); `--describe` says what it is
//! comic draw <script | png>    draw a script, or a strip again, with no model [--out x.png] [--seed N] [--renderer gpu|cpu|auto]
//! comic describe               the agent's entry, readable
//! comic sample                 the fixture script
//! comic extract <png>          the script a strip carries
//! comic icon [--out path]      the 1024 px icon the store shows
//! comic compare <a.png> <b.png> [--tolerance 3]
//! ```

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(code) = comic_agent::cli::run(&arguments) {
        std::process::exit(code);
    }
    granite_agents::AgentServer::main(granite_agents::roster![comic_agent::ComicAgent])
}
