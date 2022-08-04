#[derive(serde::Deserialize)]
pub(crate) struct Config {
	pub(crate) ssh: Ssh,

	pub(crate) services: Option<Services>,
}

impl Config {
	pub(crate) fn load() -> Result<Self, crate::Error> {
		let mut path = dirs::config_dir().ok_or("config dir not defined")?;
		path.push("opnsense-dashboard");
		path.push("config.yaml");
		let f = std::fs::File::open(path)?;
		let result = serde_yaml::from_reader(f)?;
		Ok(result)
	}
}

#[derive(serde::Deserialize)]
pub(crate) struct Services {
	#[serde(default)]
	pub(crate) builtin: Vec<String>,

	#[serde(default)]
	pub(crate) custom: Vec<CustomService>,
}

#[derive(serde::Deserialize)]
pub(crate) struct Ssh {
	pub(crate) hostname: String,
	pub(crate) username: String,
	pub(crate) identity_comment: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct CustomService {
	pub(crate) name: String,
	#[serde(flatten)]
	pub(crate) monitor: ServiceMonitor<'static>,
}

#[derive(serde::Deserialize)]
pub(crate) enum ServiceMonitor<'a> {
	#[serde(rename = "cmdline")]
	CmdLine(std::borrow::Cow<'a, str>),
	#[serde(rename = "pidfile")]
	PidFile(std::borrow::Cow<'a, str>),
}
