extern crate galah;

extern crate clap;
use clap::*;
use std::env;
use std::process;

#[macro_use]
extern crate log;
extern crate env_logger;
use log::LevelFilter;
use env_logger::Builder;
extern crate rayon;

fn main(){
    let app = build_cli();
    let matches = app.clone().get_matches();
    set_log_level(&matches, false);

    match matches.subcommand_name() {
        Some("cluster") => {
            let m = matches.subcommand_matches("cluster").unwrap();
            set_log_level(m, true);

            let num_threads = value_t!(m.value_of("threads"), usize).unwrap();
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .expect("Programming error: rayon initialised multiple times");

            let genome_fasta_files: Vec<String> = parse_list_of_genome_fasta_files(m);

            let galah = galah::cluster_argument_parsing::generate_galah_clusterer(&genome_fasta_files, &m)
                .expect("Failed to parse galah clustering arguments correctly");

            let passed_genomes = &galah.genome_fasta_paths;
            info!("Clustering {} genomes ..", passed_genomes.len());
            let clusters = galah.cluster();

            info!("Found {} genome clusters", clusters.len());


            for cluster in clusters {
                let rep_index = cluster[0];
                for genome_index in cluster {
                    println!("{}\t{}", passed_genomes[rep_index], passed_genomes[genome_index]);
                }
            }
            info!("Finished printing genome clusters");
        },
        Some("dist") => {
            let m = matches.subcommand_matches("dist").unwrap();
            set_log_level(m, true);

            let n_hashes = value_t!(m.value_of("num-hashes"), usize).unwrap();
            let kmer_length = value_t!(m.value_of("kmer-length"), u8).unwrap();

            let num_threads = value_t!(m.value_of("threads"), usize).unwrap();

            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .expect("Programming error: rayon initialised multiple times");

            info!("Reading CheckM tab table ..");
            let checkm = checkm::CheckMTabTable::read_file_path(
                m.value_of("checkm-tab-table").unwrap()
            );

            let genome_fasta_files = parse_list_of_genome_fasta_files(&m);
            
            let qualities = genome_fasta_files.iter().map(|fasta| 
                checkm.retrieve_via_fasta_path(fasta)
                    .expect(&format!("Failed to link genome fasta file {} to a CheckM quality", fasta))
                )
                .collect::<Vec<_>>();
            info!("Linked {} genomes to their CheckM quality", qualities.len());

            info!("Printing distances ..");
            galah::ani_correction::print_metaani_distances(
                &genome_fasta_files.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice(),
                qualities.as_slice(),
                n_hashes, kmer_length);
            info!("Finished");
        },
        _ => panic!("Programming error")
    }
}

fn set_log_level(matches: &clap::ArgMatches, is_last: bool) {
    let mut log_level = LevelFilter::Info;
    let mut specified = false;
    if matches.is_present("verbose") {
        specified = true;
        log_level = LevelFilter::Debug;
    }
    if matches.is_present("quiet") {
        specified = true;
        log_level = LevelFilter::Error;
    }
    if specified || is_last {
        let mut builder = Builder::new();
        builder.filter_level(log_level);
        if env::var("RUST_LOG").is_ok() {
            builder.parse_filters(&env::var("RUST_LOG").unwrap());
        }
        if builder.try_init().is_err() {
            panic!("Failed to set log level - has it been specified multiple times?")
        }
    }
    if is_last {
        info!("Cockatoo version {}", crate_version!());
    }
}

fn parse_list_of_genome_fasta_files(m: &clap::ArgMatches) -> Vec<String> {
    match m.is_present("genome-fasta-files") {
        true => {
            m.values_of("genome-fasta-files").unwrap().map(|s| s.to_string()).collect()
        },
        false => {
            if m.is_present("genome-fasta-directory") {
                let dir = m.value_of("genome-fasta-directory").unwrap();
                let paths = std::fs::read_dir(dir).unwrap();
                let mut genome_fasta_files: Vec<String> = vec!();
                let extension = m.value_of("genome-fasta-extension").unwrap();
                for path in paths {
                    let file = path.unwrap().path();
                    match file.extension() {
                        Some(ext) => {
                            if ext == extension {
                                let s = String::from(file.to_string_lossy());
                                genome_fasta_files.push(s);
                            } else {
                                info!(
                                    "Not using directory entry '{}' as a genome FASTA file, as \
                                     it does not end with the extension '{}'",
                                    file.to_str().expect("UTF8 error in filename"),
                                    extension);
                            }
                        },
                        None => {
                            info!("Not using directory entry '{}' as a genome FASTA file",
                                  file.to_str().expect("UTF8 error in filename"));
                        }
                    }
                }
                if genome_fasta_files.len() == 0 {
                    error!("Found 0 genomes from the genome-fasta-directory, cannot continue.");
                    process::exit(1);
                }
                genome_fasta_files // return
            } else {
                error!("Either a separator (-s) or path(s) to genome FASTA files \
                        (with -d or -f) must be given");
                process::exit(1);
            }
        }
    }
}


fn build_cli() -> App<'static, 'static> {
    return App::new("galah")
        .version(crate_version!())
        .author("Ben J. Woodcroft <benjwoodcroft near gmail.com>")
        .about("Metagenome assembled genomes (MAGs) dereplicator / clusterer")
        .args_from_usage("-v, --verbose       'Print extra debug logging information'
             -q, --quiet         'Unless there is an error, do not print logging information'")
        .global_setting(AppSettings::ArgRequiredElseHelp)
        // .subcommand(
        //     SubCommand::with_name("dist")
        //         .about("Calculate pairwise distances between a set of genomes")

        //         .arg(Arg::with_name("checkm-tab-table")
        //             .long("checkm-tab-table")
        //             .required(true)
        //             .help("Output of CheckM lineage_wf/taxonomy_wf/qa with --tab_table specified")
        //             .takes_value(true))
        //         .arg(Arg::with_name("genome-fasta-files")
        //             .long("genome-fasta-files")
        //             .multiple(true)
        //             .required(true)
        //             .takes_value(true))
        //         .arg(Arg::with_name("num-hashes")
        //             .long("num-hashes")
        //             .takes_value(true)
        //             .default_value("1000"))
        //         .arg(Arg::with_name("kmer-length")
        //             .long("kmer-length")
        //             .takes_value(true)
        //             .default_value("21"))
        //         .arg(Arg::with_name("threads")
        //             .short("-t")
        //             .long("threads")
        //             .default_value("1")
        //             .takes_value(true)))

        .subcommand(
            SubCommand::with_name("cluster")
                .about("Cluster FASTA files by average nucleotide identity")
                .arg(Arg::with_name("ani")
                    .long("ani")
                    .help("Average nucleotide identity threshold for clustering")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::with_name("checkm-tab-table")
                    .long("checkm-tab-table")
                    .help("Output of CheckM lineage_wf/taxonomy_wf/qa with --tab_table specified")
                    .takes_value(true))
                .arg(Arg::with_name("min-completeness")
                    .long("min-completeness")
                    .help("Genomes with less than this percentage of completeness are exluded")
                    .requires("checkm-tab-table")
                    .takes_value(true)
                    .default_value("0"))
                .arg(Arg::with_name("max-contamination")
                    .long("max-contamination")
                    .requires("checkm-tab-table")
                    .help("Genomes with greater than this percentage of contamination are exluded")
                    .takes_value(true))
                .arg(Arg::with_name("num-hashes")
                    .long("num-hashes")
                    .help("Number of hashes to use for each genome in MinHash")
                    .takes_value(true)
                    .default_value("1000"))
                .arg(Arg::with_name("kmer-length")
                    .long("kmer-length")
                    .takes_value(true)
                    .help("Kmer length to use in MinHash")
                    .default_value("21"))
                .arg(Arg::with_name("minhash-prethreshold")
                    .long("minhash-prethreshold")
                    .help("When --method minhash+fastani is specified, require at least this MinHash-derived ANI for preclustering and to avoid FastANI on distant lineages within preclusters")
                    .takes_value(true)
                    .default_value("90"))
                .arg(Arg::with_name("genome-fasta-files")
                        .short("f")
                        .long("genome-fasta-files")
                        .help("List of fasta files for clustering")
                        .multiple(true)
                        .conflicts_with("genome-fasta-directory")
                        .required_unless_one(
                            &["genome-fasta-directory"])
                        .takes_value(true))
                .arg(Arg::with_name("genome-fasta-directory")
                        .long("genome-fasta-directory")
                        .help("Directory containing fasta files for clustering")
                        .conflicts_with("genome-fasta-files")
                        .conflicts_with("single-genome")
                        .required_unless_one(
                            &["genome-fasta-files"])
                        .takes_value(true))
                .arg(Arg::with_name("genome-fasta-extension")
                        .short("x")
                        .help("File extension of FASTA files in --genome-fasta-directory")
                        .long("genome-fasta-extension")
                        // Unsure why, but uncommenting causes test failure (in
                        // coverm genome mode where this code was pasted from,
                        // not sure about here) - clap bug?
                        //.requires("genome-fasta-directory")
                        .default_value("fna")
                        .takes_value(true))

                .arg(Arg::with_name("method")
                    .long("method")
                    .possible_values(&["minhash+fastani","minhash"])
                    .default_value("minhash+fastani")
                    .help("ANI calculation method: 'minhash+fastani' for rough calculation with minhash combined with accurate calculation with FastANI, 'minhash' for minhash only.")
                    .takes_value(true))
                .arg(Arg::with_name("threads")
                    .short("-t")
                    .long("threads")
                    .help("Number of CPU threads to use")
                    .default_value("1")
                    .takes_value(true)))
            
}
