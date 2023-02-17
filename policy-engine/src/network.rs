use rhai::{def_package, plugin::*};

use crate::uri::UriPackage;

def_package! {
    pub NetworkPackage(module): UriPackage {
        combine_with_exported_module!(module, "IpAddr", ip_module);
        combine_with_exported_module!(module, "Ipv4Addr", ipv4_module);
        combine_with_exported_module!(module, "Ipv6Addr", ipv6_module);
    }
}

#[export_module]
mod ip_module {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[rhai_fn(global, get = "enum_type", pure)]
    pub fn get_type(addr: &mut IpAddr) -> ImmutableString {
        match addr {
            IpAddr::V4(..) => "v4".into(),
            IpAddr::V6(..) => "v6".into(),
        }
    }

    #[rhai_fn(global, pure)]
    pub fn is_ipv4(addr: &mut IpAddr) -> bool {
        addr.is_ipv4()
    }
    #[rhai_fn(global, pure)]
    pub fn is_ipv6(addr: &mut IpAddr) -> bool {
        addr.is_ipv6()
    }
    #[rhai_fn(global, pure)]
    pub fn is_loopback(addr: &mut IpAddr) -> bool {
        addr.is_loopback()
    }
    #[rhai_fn(global, pure)]
    pub fn is_unspecified(addr: &mut IpAddr) -> bool {
        addr.is_unspecified()
    }

    #[rhai_fn(global, pure, return_raw)]
    pub fn as_ipv4(addr: &mut IpAddr) -> Result<Ipv4Addr, Box<EvalAltResult>> {
        match addr {
            IpAddr::V4(addr) => Ok(addr.clone()),
            IpAddr::V6(..) => Err("Address is not an ipv4 address".into()),
        }
    }
    #[rhai_fn(global, pure, return_raw)]
    pub fn as_ipv6(addr: &mut IpAddr) -> Result<Ipv6Addr, Box<EvalAltResult>> {
        match addr {
            IpAddr::V4(..) => Err("Address is not an ipv6 address".into()),
            IpAddr::V6(addr) => Ok(addr.clone()),
        }
    }
}

#[export_module]
mod ipv4_module {
    use std::net::Ipv4Addr;

    pub const LOCALHOST: Ipv4Addr = Ipv4Addr::LOCALHOST;
    pub const UNSPECIFIED: Ipv4Addr = Ipv4Addr::LOCALHOST;
    pub const BROADCAST: Ipv4Addr = Ipv4Addr::LOCALHOST;

    #[rhai_fn(global, pure)]
    pub fn octets(addr: &mut Ipv4Addr) -> [u8; 4] {
        addr.octets().clone()
    }
    #[rhai_fn(global, pure)]
    pub fn is_unspecified(addr: &mut Ipv4Addr) -> bool {
        addr.is_unspecified()
    }
    #[rhai_fn(global, pure)]
    pub fn is_loopback(addr: &mut Ipv4Addr) -> bool {
        addr.is_loopback()
    }
    #[rhai_fn(global, pure)]
    pub fn is_private(addr: &mut Ipv4Addr) -> bool {
        addr.is_private()
    }
    #[rhai_fn(global, pure)]
    pub fn is_link_local(addr: &mut Ipv4Addr) -> bool {
        addr.is_link_local()
    }
    #[rhai_fn(global, pure)]
    pub fn is_mulicast(addr: &mut Ipv4Addr) -> bool {
        addr.is_multicast()
    }
    #[rhai_fn(global, pure)]
    pub fn is_broadcast(addr: &mut Ipv4Addr) -> bool {
        addr.is_broadcast()
    }
    #[rhai_fn(global, pure)]
    pub fn is_documentation(addr: &mut Ipv4Addr) -> bool {
        addr.is_documentation()
    }
}

#[export_module]
mod ipv6_module {
    use std::net::Ipv6Addr;

    pub const LOCALHOST: Ipv6Addr = Ipv6Addr::LOCALHOST;
    pub const UNSPECIFIED: Ipv6Addr = Ipv6Addr::UNSPECIFIED;

    #[rhai_fn(global, pure)]
    pub fn segments(addr: &mut Ipv6Addr) -> [u16; 8] {
        addr.segments().clone()
    }
    #[rhai_fn(global, pure)]
    pub fn octets(addr: &mut Ipv6Addr) -> [u8; 16] {
        addr.octets().clone()
    }

    #[rhai_fn(global, pure)]
    pub fn is_unspecified(addr: &mut Ipv6Addr) -> bool {
        addr.is_unspecified()
    }
    #[rhai_fn(global, pure)]
    pub fn is_loopback(addr: &mut Ipv6Addr) -> bool {
        addr.is_loopback()
    }
    #[rhai_fn(global, pure)]
    pub fn is_multicast(addr: &mut Ipv6Addr) -> bool {
        addr.is_multicast()
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::NetworkPackage;
    use crate::{eval_test, register_package};
    use rhai::{Engine, Scope};

    eval_test!(test_generic_loopback("addr": IpAddr::V4(Ipv4Addr::LOCALHOST)) -> bool | (true): "addr.is_loopback()", NetworkPackage);
    eval_test!(test_generic_unspecified("addr": IpAddr::V4(Ipv4Addr::UNSPECIFIED)) -> bool | (true): "addr.is_unspecified()", NetworkPackage);

    eval_test!(test_v4_loopback("addr": Ipv4Addr::LOCALHOST) -> bool | (true): "addr.is_loopback()", NetworkPackage);
    eval_test!(test_v4_unspecified("addr": Ipv4Addr::UNSPECIFIED) -> bool | (true): "addr.is_unspecified()", NetworkPackage);

    eval_test!(test_v6_loopback("addr": Ipv6Addr::LOCALHOST) -> bool | (true): "addr.is_loopback()", NetworkPackage);
    eval_test!(test_v6_unspecified("addr": Ipv6Addr::UNSPECIFIED) -> bool | (true): "addr.is_unspecified()", NetworkPackage);

    eval_test!(test_v6_segments("addr": IpAddr::V6(Ipv6Addr::LOCALHOST)) -> [u16; 8] | ([0, 0, 0, 0, 0, 0, 0, 1]): "addr.as_ipv6().segments()", NetworkPackage);
    eval_test!(test_v6_octets("addr": IpAddr::V6(Ipv6Addr::LOCALHOST)) -> [u8; 16] | ([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]): "addr.as_ipv6().octets()", NetworkPackage);

    eval_test!(test_v4_octets("addr": IpAddr::V4(Ipv4Addr::LOCALHOST)) -> [u8; 4] | ([127, 0, 0, 1]): "addr.as_ipv4().octets()", NetworkPackage);
}
