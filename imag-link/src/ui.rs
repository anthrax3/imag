use clap::{Arg, App, SubCommand};

pub fn build_ui<'a>(app: App<'a, 'a>) -> App<'a, 'a> {
    app
        .subcommand(SubCommand::with_name("internal")
                    .about("Add, remove and list internal links")
                    .version("0.1")
                    .subcommand(SubCommand::with_name("add")
                                .about("Add link from one entry to another (and vice-versa)")
                                .version("0.1")
                                .arg(Arg::with_name("from")
                                     .long("from")
                                     .short("f")
                                     .takes_value(true)
                                     .required(true)
                                     .help("Link from this entry"))
                                .arg(Arg::with_name("to")
                                     .long("to")
                                     .short("t")
                                     .takes_value(true)
                                     .required(true)
                                     .multiple(true)
                                     .help("Link to this entries"))
                                )

                    .subcommand(SubCommand::with_name("remove")
                            .about("Remove a link between two or more entries")
                            .version("0.1")
                            .arg(Arg::with_name("from")
                                 .long("from")
                                 .short("f")
                                 .takes_value(true)
                                 .required(true)
                                 .help("Remove Link from this entry"))
                            .arg(Arg::with_name("to")
                                 .long("to")
                                 .short("t")
                                 .takes_value(true)
                                 .required(true)
                                 .multiple(true)
                                 .help("Remove links to these entries"))
                            )

                        .arg(Arg::with_name("list")
                             .long("list")
                             .short("l")
                             .takes_value(false)
                             .required(false)
                             .help("List links to this entry"))
                    )
        .subcommand(SubCommand::with_name("external")
                    .about("Add and remove external links")
                    .version("0.1")

                    .arg(Arg::with_name("id")
                         .long("id")
                         .short("i")
                         .takes_value(true)
                         .required(true)
                         .help("Modify external link of this entry"))

                    .arg(Arg::with_name("set")
                         .long("set")
                         .short("s")
                         .takes_value(true)
                         .required(false)
                         .help("Set this URI as external link"))

                    .arg(Arg::with_name("remove")
                         .long("remove")
                         .short("r")
                         .takes_value(false)
                         .required(false)
                         .help("Remove external link"))

                    .arg(Arg::with_name("list")
                         .long("list")
                         .short("l")
                         .takes_value(false)
                         .required(false)
                         .help("List external link"))

                    .arg(Arg::with_name("show")
                         .long("show")
                         .short("s")
                         .takes_value(false)
                         .required(false)
                         .help("List external link (alias for --list)"))
                    )
}
