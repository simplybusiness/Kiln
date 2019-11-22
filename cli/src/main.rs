use clap::{App, AppSettings, Arg, SubCommand};
use futures::prelude::Future;
use shiplift::{builder::PullOptions, Docker};
use tokio::prelude::*;

use std::boxed::Box;

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

    let tool_fut = match matches.subcommand() {
        ("ruby", Some(sub_m)) => {
            match sub_m.subcommand_name() {
                Some("dependencies") => {
                    run_ruby_dependencies_tool(use_local_image)
                },
                _ => unreachable!()
            }
        },
        _ => unreachable!()
    };

    tokio::run(tool_fut);
}

fn run_ruby_dependencies_tool(use_local_image: bool) -> Box<dyn Future<Item=(), Error=()> + Send + 'static>{
    let tool_image_name = "kiln/bundler-audit:latest";
    let docker = Docker::new();
    if use_local_image {
        return Box::new(
            docker.images()
                .get(tool_image_name)
                .inspect()
                .then(move |res| {
                    match res {
                        Ok(_) => futures::future::ok(()),
                        Err(err) => {
                            match &err {
                                shiplift::errors::Error::Fault{code, message: _} if *code == 404 => eprintln!("Could not find {} locally. Quitting!", tool_image_name),
                                _  => eprintln!("{}", err)
                            };
                            futures::future::err(())
                        }
                    }
                })
            );
    } else {
        let pull_options = PullOptions::builder()
            .image(tool_image_name)
            .build();

        return Box::new(
            docker.images()
                .pull(&pull_options)
                .inspect(|item| println!("{}", item["status"].as_str().unwrap()))
                .collect()
                .then(move |res| {
                    match res {
                        Ok(_) => {
                            futures::future::ok(())
                        },
                        Err(err) => {
                            match &err {
                                shiplift::errors::Error::Fault{code, message: _} if *code == 404 => eprintln!("Could not find {} on Docker Hub. Quitting!", tool_image_name),
                                _  => eprintln!("{}", err)
                            };
                            futures::future::err(())
                        }
                    }
                })
            );
    }
}
