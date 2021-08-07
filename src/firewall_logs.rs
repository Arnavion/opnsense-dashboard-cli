#[derive(Debug)]
pub(crate) struct Logs {
	interfaces: std::collections::BTreeSet<String>,

	inner: [Option<Log>; 10],

	// Index of the newest log. Moves backwards as new logs are pushed.
	head: usize,

	previous_digest: Option<String>,
}

impl Logs {
	pub(crate) fn new(interfaces: impl IntoIterator<Item = String>) -> Self {
		let interfaces = interfaces.into_iter().collect();

		Logs {
			interfaces,
			inner: Default::default(),
			head: 0,
			previous_digest: None,
		}
	}

	pub(crate) fn update(&mut self, session: &ssh2::Session) -> Result<(), crate::Error> {
		for log in crate::ssh_exec::clog_filter_log::run(session, self.previous_digest.as_deref())?.into_iter().rev() {
			if self.previous_digest.as_deref() == Some(&log.digest) {
				continue;
			}

			self.previous_digest = Some(log.digest);

			match log.reason {
				crate::ssh_exec::clog_filter_log::LogReason::Match => (),
				crate::ssh_exec::clog_filter_log::LogReason::Other => continue,
			}

			match log.direction {
				crate::ssh_exec::clog_filter_log::LogDirection::In => (),
				crate::ssh_exec::clog_filter_log::LogDirection::Other => continue,
			}

			if !self.interfaces.contains(&log.interface) {
				continue;
			}

			let timestamp = log.timestamp;

			let interface = log.interface;

			let action = match log.action {
				crate::ssh_exec::clog_filter_log::LogAction::Block => Action::Block,
				crate::ssh_exec::clog_filter_log::LogAction::Pass => Action::Pass,
				crate::ssh_exec::clog_filter_log::LogAction::Other => continue,
			};

			let protocol = match log.ip_fields {
				crate::ssh_exec::clog_filter_log::LogIpFields::V4 { proto, src, dest } => match proto {
					crate::ssh_exec::clog_filter_log::LogV4ProtoFields::Icmp => Protocol::Icmp {
						source: std::net::IpAddr::V4(src),
						destination: std::net::IpAddr::V4(dest),
					},

					crate::ssh_exec::clog_filter_log::LogV4ProtoFields::Tcp { src_port, dest_port } => Protocol::Tcp {
						source: std::net::SocketAddr::new(std::net::IpAddr::V4(src), src_port),
						destination: std::net::SocketAddr::new(std::net::IpAddr::V4(dest), dest_port),
					},

					crate::ssh_exec::clog_filter_log::LogV4ProtoFields::Udp { src_port, dest_port } => Protocol::Udp {
						source: std::net::SocketAddr::new(std::net::IpAddr::V4(src), src_port),
						destination: std::net::SocketAddr::new(std::net::IpAddr::V4(dest), dest_port),
					},

					crate::ssh_exec::clog_filter_log::LogV4ProtoFields::Other => continue,
				},

				crate::ssh_exec::clog_filter_log::LogIpFields::V6 { proto, src, dest } => match proto {
					crate::ssh_exec::clog_filter_log::LogV6ProtoFields::Icmpv6 => Protocol::Icmp {
						source: std::net::IpAddr::V6(src),
						destination: std::net::IpAddr::V6(dest),
					},

					crate::ssh_exec::clog_filter_log::LogV6ProtoFields::Tcp { src_port, dest_port } => Protocol::Tcp {
						source: std::net::SocketAddr::new(std::net::IpAddr::V6(src), src_port),
						destination: std::net::SocketAddr::new(std::net::IpAddr::V6(dest), dest_port),
					},

					crate::ssh_exec::clog_filter_log::LogV6ProtoFields::Udp { src_port, dest_port } => Protocol::Udp {
						source: std::net::SocketAddr::new(std::net::IpAddr::V6(src), src_port),
						destination: std::net::SocketAddr::new(std::net::IpAddr::V6(dest), dest_port),
					},

					crate::ssh_exec::clog_filter_log::LogV6ProtoFields::Other => continue,
				},

				crate::ssh_exec::clog_filter_log::LogIpFields::Other => continue,
			};

			self.head = (self.head + self.inner.len() - 1) % self.inner.len();
			self.inner[self.head] = Some(Log {
				timestamp,
				interface,
				action,
				protocol,
			});
		}

		Ok(())
	}

	pub(crate) fn iter(&self) -> impl Iterator<Item = &'_ Log> {
		let (second, first) = self.inner.split_at(self.head);
		first.iter().chain(second).filter_map(Option::as_ref)
	}
}

#[derive(Debug)]
pub(crate) struct Log {
	pub(crate) timestamp: String,
	pub(crate) interface: String,
	pub(crate) action: Action,
	pub(crate) protocol: Protocol,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Action {
	Block,
	Pass,
}

impl std::fmt::Display for Action {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Action::Block => f.write_str("block"),
			Action::Pass => f.write_str("pass "),
		}
	}
}

impl std::str::FromStr for Action {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"block" => Ok(Action::Block),
			"pass" => Ok(Action::Pass),
			_ => Err(()),
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Protocol {
	Icmp { source: std::net::IpAddr, destination: std::net::IpAddr },
	Tcp { source: std::net::SocketAddr, destination: std::net::SocketAddr },
	Udp { source: std::net::SocketAddr, destination: std::net::SocketAddr },
}
