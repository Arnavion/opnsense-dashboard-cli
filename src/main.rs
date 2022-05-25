#![deny(rust_2018_idioms, warnings)]
#![deny(clippy::all, clippy::pedantic)]
#![allow(
	clippy::cast_precision_loss,
	clippy::default_trait_access,
	clippy::let_underscore_drop,
	clippy::too_many_lines,
)]

mod config;
mod opnconfig;
mod ssh_exec;

mod boot_time;
mod cpu;
mod disk;
mod firewall_logs;
mod gateway;
mod interface;
mod memory;
mod service;
mod temperature_sysctl;
mod version_info;

use std::io::Write;


// ----------------------------------------------------------------------------
// Router C ABI definitions
//
// Update these if they don't work for your router. The defaults are for an x86_64 router.

// Endianness
const ENDIANNESS: Endianness = Endianness::Little;
// const ENDIANNESS: Endianness = Endianness::Big;

// The `unsigned int` type
#[allow(non_camel_case_types)]
type c_uint = u32;

// The `unsigned long int` type
#[allow(non_camel_case_types)]
type c_ulong = u64;
// type c_ulong = u32;

// The `time_t` type
#[allow(non_camel_case_types)]
type time_t = c_ulong;

//
// ----------------------------------------------------------------------------


fn main() -> Result<(), Error> {
	let config = config::Config::load()?;


	let mut stdout = std::io::stdout().lock();

	let mut previous_terminal_size = None;


	let session = connect(&config.ssh.hostname, &config.ssh.username, Some(5000))?;


	let opnconfig = opnconfig::OpnConfig::load(&session)?;


	stdout.write_all(b"\x1B[1;1H\x1B[3J\x1B[?7l")?;

	let version_info::VersionInfo {
		version: ssh_exec::version::Version { product_arch, product_name, product_version },
		os_base_version,
	} = version_info::VersionInfo::get(&session)?;


	let mut output = vec![];


	let mut cpu = cpu::Cpu::new();

	let (boot_time, mut memory) = ssh_exec::batched_sysctls_1::run(&session)?;

	let mut disks = disk::Disk::get_all(&session)?;
	let max_disk_name_len = disks.iter().map(|disk::Disk { name, .. }| name.len()).max().unwrap_or_default();
	let max_disk_serial_number_len = disks.iter().map(|disk::Disk { serial_number, .. }| serial_number.len()).max().unwrap_or_default();

	let mut temperature_sysctls = temperature_sysctl::TemperatureSysctl::get_all(&session)?;

	let batched_sysctls_exec = ssh_exec::batched_sysctls_2::Exec::new(&temperature_sysctls[..]);

	let max_thermal_sensor_name_len =
		temperature_sysctls.iter().map(|temperature_sysctl::TemperatureSysctl { name, .. }| name)
		.chain(disks.iter().map(|disk::Disk { name, .. }| name))
		.map(String::len).max().unwrap_or_default();

	let mut interfaces = interface::Interfaces::new(opnconfig.gateway_interfaces.iter().cloned(), opnconfig.other_interfaces);
	let max_interface_name_len = interfaces.names().map(str::len).max().unwrap_or_default();

	let mut gateways = gateway::Gateways::new(opnconfig.gateways);
	let max_gateway_name_len = gateways.iter().map(|(name, _)| name.len()).max().unwrap_or_default();

	let mut services = service::Service::get_all(config.services)?;
	let max_service_name_len = services.iter().map(|service::Service { name, .. }| name.len()).max().unwrap_or_default();

	let max_firewall_log_interface_name_len = opnconfig.gateway_interfaces.iter().map(String::len).max().unwrap_or_default();
	let mut firewall_logs = firewall_logs::Logs::new(opnconfig.gateway_interfaces);


	let mut previous = std::time::SystemTime::now();


	loop {
		batched_sysctls_exec.run(&mut cpu, &mut memory, &mut temperature_sysctls[..], &session)?;

		let states_used = ssh_exec::pfctl_s_info::get_states_used(&session)?;
		let ssh_exec::netstat_m::MBufStatistics { cluster_total: mbufs_used, cluster_max: mbufs_max } = ssh_exec::netstat_m::get_mbuf_statistics(&session)?;

		let filesystems = ssh_exec::df::get_filesystems(&session)?;

		for disk in &mut disks[..] {
			disk.update(&session)?;
		}

		interfaces.update(&session)?;

		gateways.update(&session)?;

		for service in &mut services[..] {
			service.update(&session)?;
		}

		firewall_logs.update(&session)?;


		let now = std::time::SystemTime::now();
		let time_since_previous =
			now.duration_since(previous)
			.map_err(|err| format!("could not calculate time since previous iteration: {err}"))?;


		// Note:
		//
		// We don't clear the screen with [2J every time because it's slow in some terminal emulators, like tmux, and causes flickering.
		// We only do this when the screen size changes. At other times, we leave the first two lines of version info intact (since they're constant),
		// reset the cursor to just after the version info, and use [K to clear each line before we write a new one.
		//
		// The disadvantage of this method is that it relies on the number of output lines being constant.
		// There are two situations where this assumption doesn't hold:
		//
		// - The number of mounted filesystems changes. This is only an issue if you mount or unmount a filesystem dynamically.
		// - The number of IPs assigned to any interfaces changes. This should only happen if you change your interface settings.
		//
		// In either case, triggering a resize of the dashboard will fix the output.


		let terminal_size = terminal_size::terminal_size().map(|(width, height)| (width.0, height.0)).ok_or("not a TTY")?;
		if previous_terminal_size == Some(terminal_size) {
			output.extend_from_slice(b"\x1B[3;1H");
		}
		else {
			output.extend_from_slice(b"\x1B[1;1H\x1B[2J");
			writeln!(output, "Version       : {product_name} {product_version}-{product_arch}")?;
			writeln!(output, "                {os_base_version}")?;
		}


		{
			let uptime = now.duration_since(boot_time.0)?;
			let uptime = uptime.as_secs();
			write!(
				output,
				"\x1B[KUptime        : {} days {:02}:{:02}:{:02}",
				uptime / (24 * 60 * 60),
				(uptime % (24 * 60 * 60)) / (60 * 60),
				(uptime % (60 * 60)) / 60,
				uptime % 60,
			)?;
		}


		{
			output.extend_from_slice(b"\n\x1B[KCPU usage     : ");
			if let Some(cpu_usage_percent) = cpu.usage_percent() {
				let cpu_usage_color = get_color_for_usage(cpu_usage_percent);
				write!(output, "\x1B[{cpu_usage_color}m{cpu_usage_percent:5.1} %\x1B[0m")?;
			}
			else {
				output.extend_from_slice(b"    ? %");
			}
		}


		{
			let (memory_usage_percent, memory_usage_color) = usage(memory.used_pages as f32, memory.num_pages as f32);
			write!(output, "\n\x1B[KMemory usage  : \x1B[{memory_usage_color}m{memory_usage_percent:5.1} % of {} MiB\x1B[0m", memory.physical / 1_048_576)?;
		}


		{
			let states_max = (memory.physical / 10_485_760) * 1000;
			let (states_usage_percent, states_usage_color) = usage(states_used as f32, states_max as f32);
			write!(output, "\n\x1B[KStates table  : \x1B[{states_usage_color}m{states_usage_percent:5.1} % ({states_used:7} / {states_max:7})\x1B[0m")?;
		}


		{
			let (mbufs_usage_percent, mbufs_usage_color) = usage(mbufs_used as f32, mbufs_max as f32);
			write!(output, "\n\x1B[KMBUF usage    : \x1B[{mbufs_usage_color}m{mbufs_usage_percent:5.1} % ({mbufs_used:7} / {mbufs_max:7})\x1B[0m")?;
		}


		{
			output.extend_from_slice(b"\n\x1B[KDisk usage    : ");
			let max_mount_point_len = filesystems.iter().map(|filesystem| filesystem.mounted_on.len()).max().unwrap_or_default();
			for (i, filesystem) in filesystems.into_iter().enumerate() {
				let filesystem_space_used = filesystem.used_blocks;
				let filesystem_space_max = filesystem.total_blocks;
				let (filesystem_space_usage_percent, filesystem_space_usage_color) = usage(filesystem_space_used as f32, filesystem_space_max as f32);
				if i > 0 {
					output.extend_from_slice(b"\n\x1B[K                ");
				}

				write!(output,
					"\x1B[{filesystem_space_usage_color}m{:>max_mount_point_len$} : {filesystem_space_usage_percent:5.1} % of {}B\x1B[0m",
					filesystem.mounted_on,
					HumanSizeBase10(filesystem.total_blocks as f32 * 1024.),
				)?;
			}
		}


		{
			output.extend_from_slice(b"\n\x1B[KSMART status  : ");
			for (i, disk::Disk { name, serial_number, smart_passed, .. }) in disks.iter().enumerate() {
				let disk_status_color = get_color_for_up_down(*smart_passed);
				let disk_smart_status = if *smart_passed { "PASSED" } else { "FAILED" };

				if i > 0 {
					output.extend_from_slice(b"\n\x1B[K                ");
				}

				write!(output, "\x1B[{disk_status_color}m{name:>max_disk_name_len$} {serial_number:max_disk_serial_number_len$} {disk_smart_status}\x1B[0m")?;
			}
		}


		{
			output.extend_from_slice(b"\n\x1B[KTemperatures  : ");

			let thermal_sensors =
				temperature_sysctls.iter().map(|temperature_sysctl::TemperatureSysctl { name, value }| {
					let thermal_sensor_value = *value as f32 / 10. - 273.15;
					(name, thermal_sensor_value)
				})
				.chain(disks.iter().map(|disk::Disk { name, temperature, .. }| {
					let thermal_sensor_value = *temperature as f32;
					(name, thermal_sensor_value)
				}));

			for (i, (thermal_sensor_name, thermal_sensor_value)) in thermal_sensors.enumerate() {
				let thermal_sensor_color = get_color_for_temperature(thermal_sensor_value);

				if i > 0 {
					output.extend_from_slice(b"\n\x1B[K                ");
				}

				write!(output, "\x1B[{thermal_sensor_color}m{thermal_sensor_name:>max_thermal_sensor_name_len$} : {thermal_sensor_value:5.1} \u{00B0}C\x1B[0m")?;
			}
		}


		{
			output.extend_from_slice(b"\n\x1B[KInterfaces    : ");

			for (i, (interface_name, interface)) in interfaces.iter_mut().enumerate() {
				if i > 0 {
					output.extend_from_slice(b"\n\x1B[K                ");
				}

				let interface_status_color = get_color_for_up_down(interface.error.is_none());

				write!(output, "\x1B[{interface_status_color}m{interface_name:>max_interface_name_len$} : ")?;

				if let Some(interface_error) = &interface.error {
					write!(output, "{interface_error:30}")?;
				}
				else {
					match interface.speed(time_since_previous) {
						Some((interface_received_speed, interface_sent_speed)) =>
							write!(output, "{}b/s down {}b/s up ", HumanSizeBase10(interface_received_speed), HumanSizeBase10(interface_sent_speed))?,

						None =>
							output.extend_from_slice(b"    ?  b/s down     ?  b/s up "),
					}
				}

				for (i, address) in interface.addresses().enumerate() {
					if i > 0 {
						write!(
							output,
							"\n\x1B[K                \x1B[{interface_status_color}m{:>max_interface_name_len$}                                 ",
							"",
						)?;
					}

					write!(output, "{address}\x1B[0m")?;
				}
			}
		}


		{
			output.extend_from_slice(b"\n\x1B[KGateways      : ");

			for (i, (name, gateway)) in gateways.iter().enumerate() {
				if i > 0 {
					output.extend_from_slice(b"\n\x1B[K                ");
				}

				match gateway {
					Some(gateway::Gateway { latency_average, latency_stddev, ping_packet_loss }) => write!(
						output,
						"{name:>max_gateway_name_len$} : {:6.1} ms ({:6.1} ms) {ping_packet_loss:3} %",
						latency_average.as_secs_f32() * 1000.,
						latency_stddev.as_secs_f32() * 1000.,
					)?,

					None => write!(output, "{name:>max_gateway_name_len$} : dpinger is not running")?,
				}
			}
		}


		{
			output.extend_from_slice(b"\n\x1B[KServices      :");

			let num_services_per_row = (usize::from(terminal_size.0) - "Services      : ".len()) / (max_service_name_len + 2);
			let num_services_rows = (services.len() + num_services_per_row - 1) / num_services_per_row;

			for i in 0..num_services_rows {
				for j in 0..num_services_per_row {
					let service_index = i + num_services_rows * j;
					let service = match services.get(service_index) {
						Some(service) => service,
						None => break,
					};

					let service_color = get_color_for_up_down(service.is_running);

					if i > 0 && j == 0 {
						output.extend_from_slice(b"\n\x1B[K               ");
					}

					write!(output, " \x1B[{service_color}m{:max_service_name_len$}\x1B[0m ", service.name)?;
				}
			}
		}


		{
			output.extend_from_slice(b"\n\x1B[KFirewall logs : ");

			for (i, firewall_log) in firewall_logs.iter().enumerate() {
				if i > 0 {
					output.extend_from_slice(b"\n\x1B[K                ");
				}

				let firewall_log_color = get_color_for_up_down(match firewall_log.action {
					firewall_logs::Action::Block => true,
					firewall_logs::Action::Pass => false,
				});

				match firewall_log.protocol {
					firewall_logs::Protocol::Icmp { source, destination: _ } => write!(
						output,
						"\x1B[{firewall_log_color}m{} {:max_firewall_log_interface_name_len$} {}      icmp <- {source}\x1B[0m",
						firewall_log.timestamp,
						firewall_log.interface,
						firewall_log.action,
					)?,

					firewall_logs::Protocol::Tcp { source, destination } => write!(
						output,
						"\x1B[{firewall_log_color}m{} {:max_firewall_log_interface_name_len$} {} {:5}/tcp <- {}\x1B[0m",
						firewall_log.timestamp,
						firewall_log.interface,
						firewall_log.action,
						destination.port(),
						FirewallLogsSource(source),
					)?,

					firewall_logs::Protocol::Udp { source, destination } => write!(
						output,
						"\x1B[{firewall_log_color}m{} {:max_firewall_log_interface_name_len$} {} {:5}/udp <- {}\x1B[0m",
						firewall_log.timestamp,
						firewall_log.interface,
						firewall_log.action,
						destination.port(),
						FirewallLogsSource(source),
					)?,
				};
			}
		}


		stdout.write_all(&output)?;
		stdout.flush()?;
		output.clear();
		previous_terminal_size = Some(terminal_size);


		previous = now;
		std::thread::sleep(std::time::Duration::from_secs(1));
	}
}

struct Error(Box<dyn std::error::Error>, backtrace::Backtrace);

impl std::fmt::Debug for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "{}", self.0)?;

		let mut source = self.0.source();
		while let Some(err) = source {
			writeln!(f, "caused by: {err}")?;
			source = err.source();
		}

		writeln!(f)?;

		writeln!(f, "{:?}", self.1)?;

		Ok(())
	}
}

impl<E> From<E> for Error where E: Into<Box<dyn std::error::Error>> {
	fn from(err: E) -> Self {
		Error(err.into(), Default::default())
	}
}

#[allow(unused)]
#[derive(Clone, Copy, Debug)]
enum Endianness {
	Big,
	Little,
}

fn connect(hostname: &str, username: &str, timeout_ms: Option<u32>) -> Result<ssh2::Session, Error> {
	let conn = std::net::TcpStream::connect(hostname)?;

	let mut session = ssh2::Session::new()?;
	session.set_tcp_stream(conn);
	if let Some(timeout_ms) = timeout_ms {
		session.set_timeout(timeout_ms);
	}

	session.handshake()?;
	session.userauth_agent(username)?;

	Ok(session)
}

trait Parse: Sized {
	fn parse<R>(reader: &mut R) -> std::io::Result<Self> where R: std::io::Read;
}

macro_rules! impl_parse {
	($ty:ty) => {
		impl Parse for $ty {
			fn parse<R>(reader: &mut R) -> std::io::Result<Self> where R: std::io::Read {
				let mut buf = [0_u8; std::mem::size_of::<Self>()];
				let () = std::io::Read::read_exact(reader, &mut buf)?;
				let result = match ENDIANNESS {
					Endianness::Big => <$ty>::from_be_bytes(buf),
					Endianness::Little => <$ty>::from_le_bytes(buf),
				};
				Ok(result)
			}
		}
	};
}

impl_parse! { u32 }
impl_parse! { u64 }

fn usage(used: f32, max: f32) -> (f32, &'static str) {
	let usage_percent = used * 100. / max;
	let usage_color = get_color_for_usage(usage_percent);
	(usage_percent, usage_color)
}

fn get_color_for_temperature(temp: f32) -> &'static str {
	match temp {
		temp if temp < 39. => "0;34",
		temp if temp < 35. => "1;34",
		temp if temp < 40. => "1;32",
		temp if temp < 45. => "1;33",
		temp if temp < 55. => "0;33",
		temp if temp < 65. => "1;31",
		_ => "0;31",
	}
}

fn get_color_for_up_down(is_up: bool) -> &'static str {
	if is_up {
		"1;32"
	}
	else {
		"0;31"
	}
}

fn get_color_for_usage(usage: f32) -> &'static str {
	match usage {
		usage if usage < 5. => "0;34",
		usage if usage < 10. => "1;34",
		usage if usage < 25. => "1;32",
		usage if usage < 50. => "1;33",
		usage if usage < 75. => "0;33",
		usage if usage < 90. => "1;31",
		_ => "0;31",
	}
}

#[derive(Clone, Copy, Debug)]
struct HumanSizeBase10(f32);

impl std::fmt::Display for HumanSizeBase10 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let value = self.0;
		if value < 1000. {
			return write!(f, "{value:3.0}    ");
		}

		let value = value / 1000.;
		if value < 1000. {
			return write!(f, "{value:5.1} K");
		}

		let value = value / 1000.;
		if value < 1000. {
			return write!(f, "{value:5.1} M");
		}

		let value = value / 1000.;
		if value < 1000. {
			return write!(f, "{value:5.1} G");
		}

		let value = value / 1000.;
		write!(f, "{value:5.1} T")
	}
}

#[derive(Clone, Copy, Debug)]
struct FirewallLogsSource(std::net::SocketAddr);

impl std::fmt::Display for FirewallLogsSource {
	#[allow(clippy::many_single_char_names)]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self.0.ip() {
			std::net::IpAddr::V4(addr) => write!(f, "{}", addr),
			std::net::IpAddr::V6(addr) => {
				let [a, b, c, d, ..] = addr.segments();
				let addr = std::net::Ipv6Addr::new(a, b, c, d, 0, 0, 0, 0);
				write!(f, "{}/64", addr)
			},
		}
	}
}
