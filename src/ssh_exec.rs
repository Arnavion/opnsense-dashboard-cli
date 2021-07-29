pub(crate) mod batched_sysctls_1 {
	pub(crate) fn run(session: &ssh2::Session) -> Result<(crate::boot_time::BootTime, crate::memory::Memory), crate::Error> {
		let mut reader = super::exec(session, "/sbin/sysctl -b kern.boottime hw.physmem vm.stats.vm.v_page_count")?;
		let boot_time = crate::Parse::parse(&mut reader)?;
		let memory = crate::Parse::parse(&mut reader)?;
		Ok((boot_time, memory))
	}
}

pub(crate) mod batched_sysctls_2 {
	#[derive(Debug)]
	pub(crate) struct Exec {
		command: String,
	}

	impl Exec {
		pub(crate) fn new(temperature_sysctls: &[crate::temperature_sysctl::TemperatureSysctl]) -> Self {
			let mut command = "/sbin/sysctl -b vm.stats.vm.v_inactive_count vm.stats.vm.v_cache_count vm.stats.vm.v_free_count".to_owned();

			for temperature_sysctl in temperature_sysctls {
				command.push_str(&format!(" '{}'", temperature_sysctl.name));
			}

			// kern.cp_time is variable length, so it has to be at the end
			command.push_str(" kern.cp_time");

			Exec {
				command,
			}
		}

		pub(crate) fn run(
			&self,
			cpu: &mut crate::cpu::Cpu,
			memory: &mut crate::memory::Memory,
			temperature_sysctls: &mut [crate::temperature_sysctl::TemperatureSysctl],
			session: &ssh2::Session,
		) -> Result<(), crate::Error> {
			let mut reader = super::exec(session, &self.command)?;

			memory.update(&mut reader)?;

			for temperature_sysctl in temperature_sysctls {
				temperature_sysctl.update(&mut reader)?;
			}

			cpu.update(&mut reader)?;

			Ok(())
		}
	}
}

pub(crate) mod clog_filter_log {
	#[derive(Debug, serde::Deserialize)]
	pub(crate) struct Log {
		pub(crate) action: LogAction,

		#[serde(rename = "__digest__")]
		pub(crate) digest: String,

		#[serde(rename = "dir")]
		pub(crate) direction: LogDirection,

		pub(crate) interface: String,

		#[serde(flatten)]
		pub(crate) ip_fields: LogIpFields,

		pub(crate) reason: LogReason,

		#[serde(rename = "__timestamp__")]
		pub(crate) timestamp: String,
	}

	#[derive(Clone, Copy, Debug, serde::Deserialize)]
	pub(crate) enum LogAction {
		#[serde(rename = "block")]
		Block,

		#[serde(rename = "pass")]
		Pass,

		#[serde(other)]
		Other,
	}

	#[derive(Clone, Copy, Debug, serde::Deserialize)]
	pub(crate) enum LogDirection {
		#[serde(rename = "in")]
		In,

		#[serde(other)]
		Other,
	}

	#[derive(Debug, serde::Deserialize)]
	#[serde(tag = "ipversion")]
	pub(crate) enum LogIpFields {
		#[serde(rename = "4")]
		V4 {
			#[serde(flatten)]
			proto: LogV4ProtoFields,

			src: std::net::Ipv4Addr,

			#[serde(rename = "dst")]
			dest: std::net::Ipv4Addr,
		},

		#[serde(rename = "6")]
		V6 {
			#[serde(flatten)]
			proto: LogV6ProtoFields,

			src: std::net::Ipv6Addr,

			#[serde(rename = "dst")]
			dest: std::net::Ipv6Addr,
		},

		#[serde(other)]
		Other,
	}

	#[derive(Debug, serde::Deserialize)]
	#[serde(tag = "protonum")]
	pub(crate) enum LogV4ProtoFields {
		#[serde(rename = "1")]
		Icmp,

		#[serde(rename = "6")]
		Tcp {
			#[serde(rename = "srcport", deserialize_with = "deserialize_port")]
			src_port: u16,

			#[serde(rename = "dstport", deserialize_with = "deserialize_port")]
			dest_port: u16,
		},

		#[serde(rename = "17")]
		Udp {
			#[serde(rename = "srcport", deserialize_with = "deserialize_port")]
			src_port: u16,

			#[serde(rename = "dstport", deserialize_with = "deserialize_port")]
			dest_port: u16,
		},

		#[serde(other)]
		Other,
	}

	#[derive(Debug, serde::Deserialize)]
	#[serde(tag = "protonum")]
	pub(crate) enum LogV6ProtoFields {
		#[serde(rename = "58")]
		Icmpv6,

		#[serde(rename = "6")]
		Tcp {
			#[serde(rename = "srcport", deserialize_with = "deserialize_port")]
			src_port: u16,

			#[serde(rename = "dstport", deserialize_with = "deserialize_port")]
			dest_port: u16,
		},

		#[serde(rename = "17")]
		Udp {
			#[serde(rename = "srcport", deserialize_with = "deserialize_port")]
			src_port: u16,

			#[serde(rename = "dstport", deserialize_with = "deserialize_port")]
			dest_port: u16,
		},

		#[serde(other)]
		Other,
	}

	#[derive(Clone, Copy, Debug, serde::Deserialize)]
	pub(crate) enum LogReason {
		#[serde(rename = "match")]
		Match,

		#[serde(other)]
		Other,
	}

	pub(crate) fn run(session: &ssh2::Session, previous_digest: Option<&str>) -> Result<Vec<Log>, crate::Error> {
		if let Some(previous_digest) = previous_digest {
			super::read_json(super::exec(session, &format!("/usr/local/sbin/configctl filter read log 100 {}", previous_digest))?)
		}
		else {
			super::read_json(super::exec(session, "/usr/local/sbin/configctl filter read log 100")?)
		}
	}

	fn deserialize_port<'de, D>(deserializer: D) -> Result<u16, D::Error> where D: serde::Deserializer<'de> {
		struct Visitor;

		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = u16;

			fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				f.write_str("str containing a port number (u16)")
			}

			fn visit_str<E>(self, s: &str) -> Result<Self::Value, E> where E: serde::de::Error {
				s.parse().map_err(serde::de::Error::custom)
			}
		}

		deserializer.deserialize_str(Visitor)
	}
}

pub(crate) mod df {
	#[derive(Debug, serde::Deserialize)]
	struct Output {
		#[serde(rename = "storage-system-information")]
		storage_system_information: StorageSystemInformation,
	}

	#[derive(Debug, serde::Deserialize)]
	struct StorageSystemInformation {
		filesystem: Vec<Filesystem>,
	}

	#[derive(Debug, serde::Deserialize)]
	pub(crate) struct Filesystem {
		#[serde(rename = "mounted-on")]
		pub(crate) mounted_on: String,
		#[serde(rename = "total-blocks")]
		pub(crate) total_blocks: u64,
		#[serde(rename = "used-blocks")]
		pub(crate) used_blocks: u64,
	}

	pub(crate) fn get_filesystems(session: &ssh2::Session) -> Result<Vec<Filesystem>, crate::Error> {
		let Output { storage_system_information: StorageSystemInformation { filesystem } } =
			super::read_json(super::exec(session, "/bin/df -kt tmpfs,ufs --libxo json")?)?;
		Ok(filesystem)
	}
}

pub(crate) mod dpinger {
	#[derive(Debug)]
	pub(crate) struct Statistics {
		pub(crate) name: String,
		pub(crate) latency_average: std::time::Duration,
		pub(crate) latency_stddev: std::time::Duration,
		pub(crate) ping_packet_loss: crate::c_ulong,
	}

	pub(crate) fn get_statistics(session: &ssh2::Session) -> impl Iterator<Item = Result<Statistics, crate::Error>> {
		super::read_lines(super::exec(session, r#"sh -c 'for f in /var/run/dpinger_*.sock; do /usr/bin/nc -U "$f" 2>/dev/null || :; done'"#))
			.map(|line| -> Result<_, crate::Error> {
				let line = line?;

				let mut line_parts = line.split(' ');

				let name = line_parts.next().ok_or("dpinger output is malformed")?;

				let latency_average: crate::c_ulong = line_parts.next().ok_or("dpinger output is malformed")?.parse()?;
				#[allow(clippy::useless_conversion)] // c_ulong -> u64 is not necessarily identity conversion
				let latency_average = std::time::Duration::from_micros(latency_average.into());

				let latency_stddev: crate::c_ulong = line_parts.next().ok_or("dpinger output is malformed")?.parse()?;
				#[allow(clippy::useless_conversion)] // c_ulong -> u64 is not necessarily identity conversion
				let latency_stddev = std::time::Duration::from_micros(latency_stddev.into());

				let ping_packet_loss: crate::c_ulong = line_parts.next().ok_or("dpinger output is malformed")?.parse()?;

				Ok(Statistics {
					name: name.to_owned(),
					latency_average,
					latency_stddev,
					ping_packet_loss,
				})
			})
	}
}

pub(crate) mod ifconfig {
	#[derive(Debug)]
	pub(crate) struct Exec {
		command: String,
	}

	impl Exec {
		pub(crate) fn new(name: &str) -> Self {
			let command = format!("/sbin/ifconfig '{}'", name);
			Exec {
				command,
			}
		}

		pub(crate) fn run(&self, session: &ssh2::Session) -> Result<Option<String>, crate::Error> {
			let status =
				super::read_lines(super::exec(session, &self.command))
				.find_map(|line| match line {
					Ok(line) => {
						let index = line.find("status:")?;
						let value = line[(index + "status:".len())..].trim().to_owned();
						Some(Ok(value))
					},
					Err(err) => Some(Err(err)),
				})
				.transpose()?;

			if status.as_ref().map(AsRef::as_ref) == Some("active") {
				Ok(None)
			}
			else {
				Ok(status)
			}
		}
	}
}

pub(crate) mod netstat_bin {
	#[derive(Debug, serde::Deserialize)]
	struct Output {
		statistics: Statistics,
	}

	#[derive(Debug, serde::Deserialize)]
	struct Statistics {
		interface: Vec<Interface>,
	}

	#[derive(Debug, serde::Deserialize)]
	pub(crate) struct Interface {
		pub(crate) name: String,
		pub(crate) network: String,
		pub(crate) address: String,
		#[serde(rename = "received-bytes")]
		pub(crate) received_bytes: u64,
		#[serde(rename = "sent-bytes")]
		pub(crate) sent_bytes: u64,
	}

	pub(crate) fn get_interfaces(session: &ssh2::Session) -> Result<Vec<Interface>, crate::Error> {
		let Output { statistics: Statistics { interface } } = super::read_json(super::exec(session, "/usr/bin/netstat -bin --libxo json")?)?;
		Ok(interface)
	}
}

pub(crate) mod netstat_m {
	#[derive(Clone, Copy, Debug, serde::Deserialize)]
	struct Output {
		#[serde(rename = "mbuf-statistics")]
		mbuf_statistics: MBufStatistics,
	}

	#[derive(Clone, Copy, Debug, serde::Deserialize)]
	pub(crate) struct MBufStatistics {
		#[serde(rename = "cluster-max")]
		pub(crate) cluster_max: u64,
		#[serde(rename = "cluster-total")]
		pub(crate) cluster_total: u64,
	}

	pub(crate) fn get_mbuf_statistics(session: &ssh2::Session) -> Result<MBufStatistics, crate::Error> {
		let Output { mbuf_statistics } = super::read_json(super::exec(session, "/usr/bin/netstat -m --libxo json")?)?;
		Ok(mbuf_statistics)
	}
}

pub(crate) mod opnconfig {
	pub(crate) fn run(session: &ssh2::Session) -> Result<String, crate::Error> {
		super::read_string(super::read_file(session, std::path::Path::new("/conf/config.xml"))?)
	}
}

pub(crate) mod pfctl_s_info {
	pub(crate) fn get_states_used(session: &ssh2::Session) -> Result<f32, crate::Error> {
		let states_used =
			super::read_lines(super::exec(session, "/sbin/pfctl -s info"))
			.find_map(|line| match line {
				Ok(line) => {
					let index = line.find("current entries")?;
					let value = line[(index + "current entries".len())..].trim().to_owned();
					Some(Ok(value))
				},
				Err(err) => Some(Err(err)),
			})
			.ok_or("could not read state table size")??
			.parse()?;
		Ok(states_used)
	}
}

pub(crate) mod pgrep {
	#[derive(Debug)]
	pub(crate) struct Exec {
		command: String,
	}

	impl Exec {
		pub(crate) fn new(monitor: crate::config::ServiceMonitor<'_>) -> Self {
			let command = match monitor {
				crate::config::ServiceMonitor::CmdLine(cmdline) => format!("/bin/pgrep -f '^{}'", cmdline),
				crate::config::ServiceMonitor::PidFile(pidfile) => format!("/bin/pgrep -F '{}'", pidfile),
			};
			Exec {
				command,
			}
		}

		pub(crate) fn run(&self, session: &ssh2::Session) -> Result<bool, crate::Error> {
			let is_running = super::read_line(super::exec(session, &self.command))?;
			let is_running = !is_running.is_empty();
			Ok(is_running)
		}
	}
}

pub(crate) mod smartctl_a {
	#[derive(Debug)]
	pub(crate) struct Exec {
		command: String,
	}

	impl Exec {
		pub(crate) fn new(name: &str) -> Self {
			let command = format!("/usr/local/sbin/smartctl -a --json=c '/dev/{}'", name);
			Exec {
				command,
			}
		}

		pub(crate) fn run(&self, session: &ssh2::Session) -> Result<(bool, crate::c_uint), crate::Error> {
			let Output { smart_status: SmartStatus { passed }, temperature: Temperature { current } } =
				super::read_json(super::exec(session, &self.command)?)?;
			Ok((passed, current))
		}
	}

	#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
	struct Output {
		smart_status: SmartStatus,
		temperature: Temperature,
	}

	#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
	struct SmartStatus {
		passed: bool,
	}

	#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
	struct Temperature {
		current: crate::c_uint,
	}
}

pub(crate) mod smartctl_i {
	#[derive(Debug, serde::Deserialize)]
	struct Output {
		serial_number: String,
	}

	pub(crate) fn get_serial_number(name: &str, session: &ssh2::Session) -> Result<String, crate::Error> {
		let Output { serial_number } = super::read_json(super::exec(session, &format!("/usr/local/sbin/smartctl -i --json=c '/dev/{}'", name))?)?;
		Ok(serial_number)
	}
}

#[allow(non_snake_case)]
pub(crate) mod sysctl_aN {
	pub(crate) fn run(session: &ssh2::Session) -> impl Iterator<Item = Result<String, crate::Error>> {
		super::read_lines(super::exec(session, "/sbin/sysctl -aN"))
	}
}

pub(crate) mod sysctl_kern_disks {
	pub(crate) fn run(session: &ssh2::Session) -> Result<String, crate::Error> {
		super::read_line(super::exec(session, "/sbin/sysctl -n kern.disks"))
	}
}

pub(crate) mod uname_sr {
	pub(crate) fn run(session: &ssh2::Session) -> Result<String, crate::Error> {
		super::read_line(super::exec(session, "/usr/bin/uname -sr"))
	}
}

pub(crate) mod version {
	#[derive(Debug, serde::Deserialize)]
	pub(crate) struct Version {
		pub(crate) product_arch: String,
		pub(crate) product_name: String,
		pub(crate) product_version: String,
	}

	pub(crate) fn run(session: &ssh2::Session) -> Result<Version, crate::Error> {
		let version = super::read_json(super::read_file(session, std::path::Path::new("/usr/local/opnsense/version/core"))?)?;
		Ok(version)
	}
}

fn read_file(session: &ssh2::Session, path: &std::path::Path) -> Result<ssh2::Channel, crate::Error> {
	let (channel, _) = session.scp_recv(path)?;
	Ok(channel)
}

fn exec(session: &ssh2::Session, command: &str) -> Result<ssh2::Channel, crate::Error> {
	let mut channel = session.channel_session()?;
	channel.exec(command)?;
	Ok(channel)
}

fn read_json<T>(channel: ssh2::Channel) -> Result<T, crate::Error> where T: serde::de::DeserializeOwned {
	let result = serde_json::from_reader(channel)?;
	Ok(result)
}

fn read_string(mut channel: ssh2::Channel) -> Result<String, crate::Error> {
	let mut result = String::new();
	std::io::Read::read_to_string(&mut channel, &mut result)?;
	Ok(result)
}

fn read_line(channel: Result<ssh2::Channel, crate::Error>) -> Result<String, crate::Error> {
	let mut lines = read_lines(channel);
	let line = lines.next().transpose()?.unwrap_or_default();
	Ok(line)
}

fn read_lines(channel: Result<ssh2::Channel, crate::Error>) -> impl Iterator<Item = Result<String, crate::Error>> {
	enum LinesIter {
		Begin(Result<ssh2::Channel, crate::Error>),
		Read(std::io::Lines<std::io::BufReader<ssh2::Channel>>),
		Eof,
	}

	impl Iterator for LinesIter {
		type Item = Result<String, crate::Error>;

		fn next(&mut self) -> Option<Self::Item> {
			loop {
				let (next_state, result) = match std::mem::replace(self, LinesIter::Eof) {
					LinesIter::Begin(Ok(channel)) => {
						let reader = std::io::BufReader::new(channel);
						let lines = std::io::BufRead::lines(reader);
						(LinesIter::Read(lines), None)
					},

					LinesIter::Begin(Err(err)) => (LinesIter::Eof, Some(Some(Err(err)))),

					LinesIter::Read(mut lines) => {
						let result = lines.next().map(|line| line.map_err(Into::into));
						(LinesIter::Read(lines), Some(result))
					},

					LinesIter::Eof => (LinesIter::Eof, Some(None)),
				};

				*self = next_state;

				if let Some(result) = result {
					return result;
				}
			}
		}
	}

	LinesIter::Begin(channel)
}
