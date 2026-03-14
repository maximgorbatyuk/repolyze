use clap::Command;

fn main() {
    let _ = Command::new("repolyze")
        .about("Repository analytics for local Git repositories")
        .get_matches();
}
