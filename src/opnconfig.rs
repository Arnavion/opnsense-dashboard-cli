#[derive(Debug)]
pub(crate) struct OpnConfig {
	pub(crate) gateway_interfaces: std::collections::BTreeSet<String>,
	pub(crate) other_interfaces: Vec<String>,
	pub(crate) gateways: Vec<Gateway>,
}

#[derive(Debug)]
pub(crate) struct Gateway {
	pub(crate) name: String,
}

impl OpnConfig {
	pub(crate) fn load(session: &ssh2::Session) -> Result<Self, crate::Error> {
		let opnconfig = crate::ssh_exec::opnconfig::run(session)?;
		let opnconfig = roxmltree::Document::parse(&opnconfig)?;
		let mut opnconfig: OpnSense<'_> = opnconfig.root_element().try_into()?;

		let mut gateway_interfaces: std::collections::BTreeSet<_> = Default::default();
		let mut gateways = vec![];

		for (&gateway_name, &gateway_interface) in &opnconfig.gateways.0 {
			let &r#if =
				opnconfig.interfaces.0
				.get(gateway_interface)
				.ok_or_else(|| format!("gateway {} is defined on interface {} but this interface does not exist", gateway_name, gateway_interface))?;
			gateway_interfaces.insert(r#if.to_owned());

			gateways.push(Gateway {
				name: gateway_name.to_owned(),
			});
		}

		for gateway_interface in opnconfig.gateways.0.into_values() {
			let _ = opnconfig.interfaces.0.remove(gateway_interface);
		}

		let other_interfaces = opnconfig.interfaces.0.into_values().map(ToOwned::to_owned).collect();

		let result = OpnConfig {
			gateway_interfaces,
			other_interfaces,
			gateways,
		};

		Ok(result)
	}
}

#[derive(Debug)]
struct OpnSense<'input> {
	interfaces: Interfaces<'input>,
	gateways: Gateways<'input>,
}

impl<'input> TryFrom<roxmltree::Node<'input, 'input>> for OpnSense<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let interfaces_tag_name: roxmltree::ExpandedName<'_, '_> = "interfaces".into();
		let gateways_tag_name: roxmltree::ExpandedName<'_, '_> = "gateways".into();

		let mut interfaces = None;
		let mut gateways = None;

		for child in node.children() {
			let child_tag_name = child.tag_name();
			if child_tag_name == interfaces_tag_name {
				interfaces = Some(child.try_into()?);
			}
			else if child_tag_name == gateways_tag_name {
				gateways = Some(child.try_into()?);
			}
		}

		let interfaces = interfaces.ok_or("interfaces not found in config.xml")?;
		let gateways = gateways.ok_or("gateways not found in config.xml")?;

		Ok(OpnSense {
			interfaces,
			gateways,
		})
	}
}

#[derive(Debug)]
struct Interfaces<'input>(std::collections::BTreeMap<&'input str, &'input str>);

impl<'input> TryFrom<roxmltree::Node<'input, 'input>> for Interfaces<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let inner: Result<_, crate::Error> =
			node.children()
			.filter_map(|child|
				if child.is_element() {
					let Interface { name, r#if, internal_dynamic } = match child.try_into() {
						Ok(interface) => interface,
						Err(err) => return Some(Err(err)),
					};
					if internal_dynamic {
						None
					}
					else {
						Some(Ok((name, r#if)))
					}
				}
				else {
					None
				})
			.collect();
		let inner = inner?;

		Ok(Interfaces(inner))
	}
}

#[derive(Debug)]
struct Interface<'input> {
	name: &'input str,
	r#if: &'input str,
	internal_dynamic: bool,
}

impl<'input> TryFrom<roxmltree::Node<'input, 'input>> for Interface<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let if_tag_name: roxmltree::ExpandedName<'_, '_> = "if".into();
		let internal_dynamic_tag_name: roxmltree::ExpandedName<'_, '_> = "internal_dynamic".into();

		let name = node.tag_name().name();

		let r#if = node.children().find(|node| node.tag_name() == if_tag_name).ok_or("interfaces.*.if not found in config.xml")?;
		let r#if = r#if.text().ok_or("interfaces.*.if is not a text node")?;

		let internal_dynamic = node.children().find(|node| node.tag_name() == internal_dynamic_tag_name);
		let internal_dynamic = internal_dynamic.and_then(|internal_dynamic| internal_dynamic.text()).map_or(false, |internal_dynamic| internal_dynamic == "1");

		Ok(Interface {
			name,
			r#if,
			internal_dynamic,
		})
	}
}

#[derive(Debug)]
struct Gateways<'input>(std::collections::BTreeMap<&'input str, &'input str>);

impl<'input> TryFrom<roxmltree::Node<'input, 'input>> for Gateways<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let gateway_item_tag_name: roxmltree::ExpandedName<'_, '_> = "gateway_item".into();

		let inner: Result<_, crate::Error> =
			node.children()
			.filter_map(|child|
				if child.tag_name() == gateway_item_tag_name {
					let GatewayItem { name, interface } = match child.try_into() {
						Ok(gateway) => gateway,
						Err(err) => return Some(Err(err)),
					};
					Some(Ok((name, interface)))
				}
				else {
					None
				})
			.collect();
		let inner = inner?;

		Ok(Gateways(inner))
	}
}

#[derive(Debug)]
struct GatewayItem<'input> {
	name: &'input str,
	interface: &'input str,
}

impl<'input> TryFrom<roxmltree::Node<'input, 'input>> for GatewayItem<'input> {
	type Error = crate::Error;

	fn try_from(node: roxmltree::Node<'input, 'input>) -> Result<Self, Self::Error> {
		let name_tag_name: roxmltree::ExpandedName<'_, '_> = "name".into();
		let interface_tag_name: roxmltree::ExpandedName<'_, '_> = "interface".into();

		let name = node.children().find(|node| node.tag_name() == name_tag_name).ok_or("gateways.gateway_item.name not found in config.xml")?;
		let name = name.text().ok_or("gateways.gateway_item.name is not a text node")?;

		let interface = node.children().find(|node| node.tag_name() == interface_tag_name).ok_or("gateways.gateway_item.interface not found in config.xml")?;
		let interface = interface.text().ok_or("gateways.gateway_item.interface is not a text node")?;

		Ok(GatewayItem {
			name,
			interface,
		})
	}
}
