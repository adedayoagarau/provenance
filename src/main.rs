use clap::Parser;
use tracing::info;

mod forensics;
mod analysis;
mod identity;
mod scoring;
mod crypto;
mod utils;

#[derive(Parser, Debug)]
#[command(name = "provenance", version, about = "Forensic authorship verification platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Analyze a document for authorship verification
    Analyze {
        /// Path to the document to analyze
        #[arg(short, long)]
        file: String,

        /// Path to an author profile to compare against
        #[arg(short, long)]
        profile: Option<String>,
    },

    /// Build an author profile from verified writing samples
    Profile {
        /// Directory containing verified writing samples
        #[arg(short, long)]
        samples: String,

        /// Name for the author profile
        #[arg(short, long)]
        name: String,
    },

    /// Verify file integrity and metadata
    Forensics {
        /// Path to the file to examine
        #[arg(short, long)]
        file: String,
    },
}

fn main() -> anyhow::Result<()> {
    utils::logging::init();

    let cli = Cli::parse();
    info!("Provenance v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Analyze { file, profile } => {
            info!(file = %file, "Starting document analysis");
            let report = provenance::analyze(&file, profile.as_deref())?;
            println!("{report}");
        }
        Commands::Profile { samples, name } => {
            info!(name = %name, "Building author profile");
            let profile = provenance::build_profile(&samples, &name)?;
            println!("Profile created: {profile}");
        }
        Commands::Forensics { file } => {
            info!(file = %file, "Running file forensics");
            let report = provenance::run_forensics(&file)?;
            println!("{report}");
        }
    }

    Ok(())
}
