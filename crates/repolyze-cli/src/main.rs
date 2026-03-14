use clap::Command;

fn main() -> anyhow::Result<()> {
    let matches = Command::new("repolyze")
        .about("Repository analytics for local Git repositories")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(Command::new("tui").about("Launch the interactive TUI"))
        .get_matches();

    match matches.subcommand() {
        Some(("tui", _)) => repolyze_tui::run()?,
        None => repolyze_tui::run()?,
        _ => unreachable!(),
    }

    Ok(())
}
