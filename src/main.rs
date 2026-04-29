fn main() {
    if let Err(error) = pmenu::cli::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
