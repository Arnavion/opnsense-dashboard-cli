#[derive(Debug)]
pub(crate) struct VersionInfo {
	pub(crate) version: crate::ssh_exec::version::Version,
	pub(crate) os_base_version: String,
}

impl VersionInfo {
	pub(crate) fn get(session: &ssh2::Session) -> Result<Self, crate::Error> {
		let version = crate::ssh_exec::version::run(session)?;

		let os_base_version = crate::ssh_exec::uname_sr::run(session)?;

		Ok(VersionInfo {
			version,
			os_base_version,
		})
	}
}
