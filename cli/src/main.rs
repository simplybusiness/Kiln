use clap::{App, AppSettings, Arg, SubCommand};

fn main() {
    let matches = App::new("Kiln CLI")
        .setting(AppSettings::SubcommandRequired)
        .arg(Arg::with_name("use-local-image")
            .long("use-local-image")
            .help("Do not try and pull the latest version of a tool image. Useful for development and scanning without network access"))
        .subcommand(SubCommand::with_name("ruby")
            .about("perform security testing of Ruby based projects")
            .setting(AppSettings::SubcommandRequired)
            .subcommand(SubCommand::with_name("dependencies")
                .about("Use Bundler-audit to find known vulnerabilities in project dependencies")
            )
        ).get_matches();

    let use_local_image = matches.is_present("use-local-image");

    match matches.subcommand() {
        ("ruby", Some(sub_m)) => {
            match sub_m.subcommand_name() {
                Some("dependencies") => {
                    run_ruby_dependencies_tool(use_local_image);
                },
                _ => unreachable!()
            }
        },
        _ => unreachable!()
    }
}

fn run_ruby_dependencies_tool(use_local_image: bool) {
    println!("{}", use_local_image)
}
