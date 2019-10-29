use clap::{Arg, App}; 

fn main() {
    let matches = App::new("Kiln data forwarder")
			.arg(Arg::with_name("tool_name")
				.help("Name of the security tool run")
				.long("tool-name")
				.required(true)
				.takes_value(true)
				.value_name("TOOL_NAME")
			)
			.arg(Arg::with_name("tool_version")
				.help("Version of the security tool run")
				.long("tool-version")
				.takes_value(true)
				.value_name("TOOL_VERSION")
			)
			.arg(Arg::with_name("tool_output_path")
				.help("Path to the output of the tool")
				.required(true) 
				.long("tool-output-path")
				.takes_value(true)
				.value_name("PATH")
			)
			.arg(Arg::with_name("endpoint_url")
				.help("kiln data collector URL")
				.long("endpoint-url")
				.required(true)
				.takes_value(true)
				.value_name("URL")
			)
			.arg(Arg::with_name("start_time")
				.help("Start time of tool execution as a RFC3339 timestamp")
				.long("start-time")
				.required(true)
				.takes_value(true)
				.value_name("TIMESTAMP")
			)
			.arg(Arg::with_name("end_time")
				.help("End time of tool execution as a RFC3339 timestamp")
				.long("end-time")
				.required(true)
				.takes_value(true)
				.value_name("TIMESTAMP")
			)
			.arg(Arg::with_name("output_format")
				.help("Output format of the tool run")
				.long("output-format")
				.required(true)
				.takes_value(true)
				.value_name("OUTPUT-FORMAT")
				.possible_values(&["JSON","Plaintext"])			
			)
			.arg(Arg::with_name("env")
				.help("Environment for the tool run")
				.long("env")
				.required(true)
				.takes_value(true)
				.value_name("ENV")
				.possible_values(&["Local","CI"])			
			)
			.arg(Arg::with_name("app_name")
				.help("Name of the application on which tool was run")
				.long("app-name")
				.required(true)
				.takes_value(true)
				.value_name("APPNAME")
			).get_matches();
}
