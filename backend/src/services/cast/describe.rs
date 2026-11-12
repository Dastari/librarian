//! What a receiver says about itself when it is discovered: its name, maker and
//! model, a stable identifier, and the protocol details shown under "How it was
//! found" in Settings. Google Cast receivers say it in their mDNS TXT record;
//! DLNA renderers in the UPnP device description their SSDP reply points to.

use std::collections::BTreeMap;

use quick_xml::Reader;
use quick_xml::events::Event;
use serde::Serialize;

use super::CastDeviceType;

/// The receiver protocol, as stored in `discoveryDetails.protocol`.
pub const PROTOCOL_GOOGLE_CAST: &str = "googleCast";
pub const PROTOCOL_DLNA: &str = "dlna";

/// The JSON kept in `CastDevice.discoveryDetails`. Every field is optional so
/// what one protocol does not report is simply left out.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDetails {
    pub protocol: &'static str,
    /// The name exactly as the receiver reported it, before tidying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_number: Option<String>,
    /// mDNS: the advertised service instance and host.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// Every address the receiver answered from.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<String>,
    /// Google Cast: what it can do, decoded from the `ca` bitmask.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<&'static str>,
    /// Google Cast: the app it was showing when it was found (`rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_app: Option<String>,
    /// Google Cast: the raw TXT record.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub txt: BTreeMap<String, String>,
    /// DLNA: the device description and the SSDP headers that pointed to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usn: Option<String>,
    /// DLNA: the UPnP services it offers, such as `AVTransport:1`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<String>,
    /// DLNA: why the description could not be read, when it could not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_error: Option<String>,
}

impl DiscoveryDetails {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

/// Google Cast capability bits in the TXT `ca` value.
const CAST_CAPABILITIES: [(u32, &str); 5] = [
    (1, "Video out"),
    (2, "Video in"),
    (4, "Audio out"),
    (8, "Audio in"),
    (32, "Speaker group"),
];

pub fn cast_capabilities(ca: Option<&str>) -> Vec<&'static str> {
    let Some(bits) = ca.and_then(|value| value.trim().parse::<u32>().ok()) else {
        return Vec::new();
    };
    CAST_CAPABILITIES
        .iter()
        .filter(|(bit, _)| bits & bit != 0)
        .map(|(_, label)| *label)
        .collect()
}

/// Anything that answers on `_googlecast._tcp` is a Cast receiver: TVs with
/// Chromecast built in and Google TV sets report their own model names, so the
/// model is not required to say "Chromecast". A receiver that advertises audio
/// out but no video out (a Nest speaker, a Chromecast Audio, a speaker group)
/// is audio-only.
pub fn google_cast_device_type(model: Option<&str>, ca: Option<&str>) -> CastDeviceType {
    if model.is_some_and(|model| model.to_ascii_lowercase().contains("chromecast audio")) {
        return CastDeviceType::ChromecastAudio;
    }
    match ca.and_then(|value| value.trim().parse::<u32>().ok()) {
        Some(bits) if bits & 1 == 0 && bits & 4 != 0 => CastDeviceType::ChromecastAudio,
        _ => CastDeviceType::Chromecast,
    }
}

/// A readable name from an mDNS instance name such as
/// `Chromecast-Ultra-3f8c…a1._googlecast._tcp.local.`, for a receiver whose TXT
/// record has no `fn`.
pub fn name_from_service_instance(fullname: &str) -> String {
    let instance = fullname
        .split_once("._googlecast")
        .map_or(fullname, |(instance, _)| instance);
    // Drop a trailing `-<32 hex digits>` identifier.
    let instance = match instance.rsplit_once('-') {
        Some((head, tail)) if tail.len() >= 16 && tail.chars().all(|c| c.is_ascii_hexdigit()) => {
            head
        }
        _ => instance,
    };
    let name = instance.replace('-', " ");
    let name = name.trim();
    if name.is_empty() {
        "Cast receiver".to_string()
    } else {
        name.to_string()
    }
}

/// Samsung prefixes its renderers' names with a bracketed category such as
/// `[TV]` or `[AV]`; the rest is the name people know the set by.
pub fn tidy_friendly_name(name: &str) -> String {
    let name = name.trim();
    if let Some(rest) = name.strip_prefix('[')
        && let Some((tag, rest)) = rest.split_once(']')
        && tag.len() <= 6
        && !rest.trim().is_empty()
    {
        return rest.trim().to_string();
    }
    name.to_string()
}

/// The parts of a UPnP device description that name the root device.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpnpDescription {
    pub friendly_name: Option<String>,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
    pub model_number: Option<String>,
    pub model_description: Option<String>,
    pub udn: Option<String>,
    /// Service types without the `urn:schemas-upnp-org:service:` prefix.
    pub services: Vec<String>,
}

/// Reads the root `<device>` of a UPnP description. Embedded devices in a
/// `<deviceList>` are skipped: the root is the one the SSDP reply announced.
pub fn parse_upnp_description(xml: &str) -> anyhow::Result<UpnpDescription> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut path: Vec<String> = Vec::new();
    let mut text = String::new();
    let mut out = UpnpDescription::default();
    loop {
        match reader.read_event()? {
            Event::Start(element) => {
                path.push(element.name().local_name().as_ref().to_string());
                text.clear();
            }
            Event::Text(value) => {
                if let Ok(value) = quick_xml::escape::unescape(&value) {
                    text.push_str(&value);
                }
            }
            Event::CData(value) => text.push_str(&value),
            Event::GeneralRef(reference) => {
                if let Ok(Some(character)) = reference.resolve_char_ref() {
                    text.push(character);
                } else if let Some(resolved) =
                    quick_xml::escape::resolve_predefined_entity(&reference)
                {
                    text.push_str(resolved);
                }
            }
            Event::End(_) => {
                let value = text.trim();
                let names: Vec<&str> = path.iter().map(String::as_str).collect();
                let field = match names.as_slice() {
                    ["root", "device", field] => Some(*field),
                    _ => None,
                };
                if !value.is_empty() {
                    let value = value.to_string();
                    match field {
                        Some("friendlyName") => out.friendly_name = Some(value),
                        Some("manufacturer") => out.manufacturer = Some(value),
                        Some("modelName") => out.model_name = Some(value),
                        Some("modelNumber") => out.model_number = Some(value),
                        Some("modelDescription") => out.model_description = Some(value),
                        Some("UDN") => out.udn = Some(value),
                        _ => {
                            if names.as_slice()
                                == ["root", "device", "serviceList", "service", "serviceType"]
                            {
                                let short = value
                                    .rsplit_once(":service:")
                                    .map_or(value.as_str(), |(_, short)| short)
                                    .to_string();
                                out.services.push(short);
                            }
                        }
                    }
                }
                path.pop();
                text.clear();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

/// The `uuid:…` part of an SSDP `USN` such as
/// `uuid:a32e…::urn:schemas-upnp-org:device:MediaRenderer:1`.
pub fn udn_from_usn(usn: &str) -> Option<String> {
    let udn = usn.split("::").next()?.trim();
    udn.starts_with("uuid:").then(|| udn.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMSUNG: &str = r#"<?xml version="1.0"?>
<root xmlns="urn:schemas-upnp-org:device-1-0" xmlns:sec="http://www.sec.co.kr/dlna">
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
    <friendlyName>[TV] Samsung 8 Series (55)</friendlyName>
    <manufacturer>Samsung Electronics</manufacturer>
    <modelDescription>Samsung TV DMR</modelDescription>
    <modelName>UA55KS8000</modelName>
    <modelNumber>AllShare1.0</modelNumber>
    <UDN>uuid:a32ea4b3-4300-4b65-aa7e-5fae01ce9086</UDN>
    <sec:deviceID/>
    <serviceList>
      <service><serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType></service>
      <service><serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType></service>
      <service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType></service>
    </serviceList>
    <deviceList>
      <device><friendlyName>Embedded</friendlyName><modelName>Inner</modelName></device>
    </deviceList>
  </device>
</root>"#;

    #[test]
    fn reads_the_root_device_of_a_samsung_description() {
        let description = parse_upnp_description(SAMSUNG).unwrap();
        assert_eq!(
            description.friendly_name.as_deref(),
            Some("[TV] Samsung 8 Series (55)")
        );
        assert_eq!(
            description.manufacturer.as_deref(),
            Some("Samsung Electronics")
        );
        assert_eq!(description.model_name.as_deref(), Some("UA55KS8000"));
        assert_eq!(
            description.model_description.as_deref(),
            Some("Samsung TV DMR")
        );
        assert_eq!(
            description.udn.as_deref(),
            Some("uuid:a32ea4b3-4300-4b65-aa7e-5fae01ce9086")
        );
        assert_eq!(
            description.services,
            ["RenderingControl:1", "ConnectionManager:1", "AVTransport:1"]
        );
    }

    #[test]
    fn keeps_entities_in_names() {
        let xml =
            "<root><device><friendlyName>Tom &amp; Jo&#39;s TV</friendlyName></device></root>";
        let description = parse_upnp_description(xml).unwrap();
        assert_eq!(description.friendly_name.as_deref(), Some("Tom & Jo's TV"));
    }

    #[test]
    fn tidies_samsung_category_prefixes_only() {
        assert_eq!(
            tidy_friendly_name("[TV] Samsung 8 Series (55)"),
            "Samsung 8 Series (55)"
        );
        assert_eq!(tidy_friendly_name("Living Room TV"), "Living Room TV");
        assert_eq!(tidy_friendly_name("[TV]"), "[TV]");
        assert_eq!(
            tidy_friendly_name("[Very long bracket] TV"),
            "[Very long bracket] TV"
        );
    }

    #[test]
    fn names_from_service_instances_drop_the_identifier() {
        assert_eq!(
            name_from_service_instance(
                "Chromecast-Ultra-3f8c0e2d9b7a41c5a1b2c3d4e5f60718._googlecast._tcp.local."
            ),
            "Chromecast Ultra"
        );
        assert_eq!(
            name_from_service_instance("Google-TV-Streamer._googlecast._tcp.local."),
            "Google TV Streamer"
        );
    }

    #[test]
    fn google_cast_receivers_are_playable_whatever_their_model() {
        assert_eq!(
            google_cast_device_type(Some("TCL TV"), Some("463365")),
            CastDeviceType::Chromecast
        );
        assert_eq!(
            google_cast_device_type(Some("Google TV Streamer"), None),
            CastDeviceType::Chromecast
        );
        assert_eq!(
            google_cast_device_type(Some("Nest Audio"), Some("199172")),
            CastDeviceType::ChromecastAudio
        );
        assert_eq!(
            google_cast_device_type(Some("Chromecast Audio"), Some("2052")),
            CastDeviceType::ChromecastAudio
        );
        assert_eq!(cast_capabilities(Some("5")), ["Video out", "Audio out"]);
    }

    #[test]
    fn udn_comes_from_the_usn() {
        assert_eq!(
            udn_from_usn("uuid:a32e-1::urn:schemas-upnp-org:device:MediaRenderer:1").as_deref(),
            Some("uuid:a32e-1")
        );
        assert_eq!(udn_from_usn("urn:x"), None);
    }
}
