#[derive(Debug)]
pub(crate) struct Service {
	pub(crate) name: String,
	is_running_exec: crate::ssh_exec::pgrep::Exec,
	pub(crate) is_running: bool,
}

impl Service {
	pub(crate) fn get_all(services: Option<crate::config::Services>) -> Result<Box<[Self]>, crate::Error> {
		let (builtin_services, custom_services) = match services {
			Some(crate::config::Services { builtin, custom }) => (Some(builtin), Some(custom)),
			None => (None, None),
		};

		let result: Result<Box<[_]>, crate::Error> =
			builtin_services.into_iter()
			.flatten()
			.map(|name| -> Result<_, crate::Error> {
				let monitor = match &*name {
					"configd" => crate::config::ServiceMonitor::PidFile("/var/run/configd.pid".into()),
					"dhcpd" => crate::config::ServiceMonitor::CmdLine("/usr/local/sbin/dhcpd -user dhcpd ".into()),
					"dhcpd6" => crate::config::ServiceMonitor::CmdLine("/usr/local/sbin/dhcpd -6 -user dhcpd ".into()),
					"ntpd" => crate::config::ServiceMonitor::CmdLine("/usr/local/sbin/ntpd ".into()),
					"openssh" => crate::config::ServiceMonitor::PidFile("/var/run/sshd.pid".into()),
					"radvd" => crate::config::ServiceMonitor::PidFile("/var/run/radvd.pid".into()),
					"syslog-ng" => crate::config::ServiceMonitor::PidFile("/var/run/syslog-ng.pid".into()),
					"syslogd" => crate::config::ServiceMonitor::PidFile("/var/run/syslog.pid".into()),
					"unbound" => crate::config::ServiceMonitor::PidFile("/var/run/unbound.pid".into()),
					name => return Err(format!("{name:?} is not recognized as a built-in service").into()),
				};
				Ok((name, monitor))
			})
			.chain(
				custom_services.into_iter()
				.flatten()
				.map(|crate::config::CustomService { name, monitor }| Ok::<_, crate::Error>((name, monitor)))
			)
			.map(|service| {
				let (name, monitor) = service?;
				Ok(Service {
					name,
					is_running_exec: crate::ssh_exec::pgrep::Exec::new(monitor),
					is_running: false,
				})
			})
			.collect();
		let mut result = result?;
		result.sort_by(|service1, service2| service1.name.cmp(&service2.name));
		Ok(result)
	}

	pub(crate) fn update(&mut self, session: &ssh2::Session) -> Result<(), crate::Error> {
		self.is_running = self.is_running_exec.run(session)?;
		Ok(())
	}
}
