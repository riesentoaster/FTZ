use fuzzer::runner::fuzz;

fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .init();

    fuzz();
    // let opt = Cli::parse();
    // let packets = connect_to_zephyr(
    //     b"Hello, World!",
    //     opt.zephyr_exec_dir(),
    //     opt.zephyr_out_dir(),
    //     0,
    //     NETWORK_SHMEM_SIZE,
    //     Duration::from_secs(10),
    // )
    // .unwrap();

    // let mut pcap_file = File::create(PCAP_PATH).unwrap();
    // write_pcap(
    //     &packets.iter().map(|(d, p)| (d, p)).collect::<Vec<_>>(),
    //     &mut pcap_file,
    // )
    // .unwrap();
}
