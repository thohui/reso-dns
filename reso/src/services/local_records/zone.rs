use std::collections::HashMap;

use reso_dns::{DnsRecord, RecordType, domain_name::DomainName, message::DnsRecordData};

/// Cname chain depth limit.
const MAX_CNAME_CHAIN: usize = 8;

/// A resolved local record ready to be served as a DNS answer.
#[derive(Debug, Clone)]
pub struct ResolvedRecord {
    pub record: DnsRecord,
}

/// Result of a local record lookup.
#[derive(Debug)]
pub enum LocalLookup {
    /// Records answering the query.
    Answer(Vec<ResolvedRecord>),
    /// The name is local, but has no address records of the requested type.
    NoData,
    /// Not answered locally, forward upstream.
    NotFound,
}

/// Records of a single local name or wildcard, grouped by type.
#[derive(Debug, Default)]
struct NameRecords {
    by_type: HashMap<RecordType, Vec<ResolvedRecord>>,
}

impl NameRecords {
    fn push(&mut self, record: ResolvedRecord) {
        self.by_type.entry(record.record.record_type).or_default().push(record);
    }

    fn resolve(&self, qtype: RecordType) -> LocalLookup {
        if let Some(records) = self.by_type.get(&qtype) {
            return LocalLookup::Answer(records.clone());
        }

        // cname also answers A/AAAA (RFC 1034 3.6.2)
        if matches!(qtype, RecordType::A | RecordType::AAAA)
            && let Some(records) = self.by_type.get(&RecordType::CNAME)
        {
            return LocalLookup::Answer(records.clone());
        }

        // NODATA for address types only, everything else goes upstream
        if matches!(
            qtype,
            RecordType::A | RecordType::AAAA | RecordType::SVCB | RecordType::HTTPS
        ) {
            LocalLookup::NoData
        } else {
            LocalLookup::NotFound
        }
    }
}

/// An existing name, can be an empty non-terminal (RFC 4592 2.2.2).
#[derive(Debug, Default)]
struct ZoneNode {
    /// Empty for an empty non-terminal.
    records: NameRecords,
    /// Records of `*.<name>`.
    wildcard: Option<NameRecords>,
}

// *.home      A 10.0.0.1
// nas.home    A 10.0.0.2
// a.b.home    A 10.0.0.3
//
// home
// ├── *       wildcard
// ├── nas     nas.home
// └── b       b.home, empty non-terminal
//     └── a   a.b.home

/// Local records, wildcards follow RFC 4592.
#[derive(Debug, Default)]
pub struct LocalZone {
    /// Keyed by normalized name.
    nodes: HashMap<Box<str>, ZoneNode>,
}

impl LocalZone {
    pub fn insert_exact(&mut self, record: ResolvedRecord) {
        let name = record.record.name.clone();
        self.node_mut(name.as_str()).records.push(record);
    }

    /// Adds the record to `*.<owner>`.
    pub fn insert_wildcard(&mut self, record: ResolvedRecord) {
        let name = record.record.name.clone();
        self.node_mut(name.as_str())
            .wildcard
            .get_or_insert_with(NameRecords::default)
            .push(record);
    }

    pub fn lookup(&self, qname: &DomainName, qtype: RecordType) -> LocalLookup {
        let mut answers = match self.lookup_name(qname, qtype) {
            LocalLookup::Answer(answers) => answers,
            other => return other,
        };

        // cname queries get the cname only
        if qtype == RecordType::CNAME {
            return LocalLookup::Answer(answers);
        }

        // cnames are followed to their target (RFC 1034 4.3.2)
        // alias.home -> alias.home CNAME host.home, host.home A 10.0.0.1
        // example.com target -> chain ends at the cname
        // TODO: add support for cnames pointing to external names. Some stub resolvers have support
        // for this and make the additional query to reso, but others can fail and never send the
        // query to resolve the cname.
        for _ in 0..MAX_CNAME_CHAIN {
            let Some(target) = cname_target(&answers) else {
                break;
            };
            let LocalLookup::Answer(records) = self.lookup_name(&target, qtype) else {
                break;
            };
            answers.extend(records);
        }

        LocalLookup::Answer(answers)
    }

    fn lookup_name(&self, qname: &DomainName, qtype: RecordType) -> LocalLookup {
        let name = qname.as_str();

        // existing names never hit a wildcard (RFC 4592 2.2)
        // nas.home -> 10.0.0.2, not 10.0.0.1
        // empty non-terminals go upstream, we don't own them
        // b.home -> upstream
        if let Some(node) = self.nodes.get(name) {
            if node.records.by_type.is_empty() {
                return LocalLookup::NotFound;
            }
            return node.records.resolve(qtype);
        }

        // only the closest encloser's wildcard counts (RFC 4592 3.3.1)
        // x.home   -> encloser home, *.home -> 10.0.0.1
        // x.b.home -> encloser b.home, no *.b.home -> upstream
        let Some(encloser) = ancestors(name).find_map(|suffix| self.nodes.get(suffix)) else {
            return LocalLookup::NotFound;
        };
        let Some(wildcard) = &encloser.wildcard else {
            return LocalLookup::NotFound;
        };

        match wildcard.resolve(qtype) {
            // owner becomes the qname (RFC 4592 3.3.3), x.home not *.home
            LocalLookup::Answer(records) => LocalLookup::Answer(
                records
                    .into_iter()
                    .map(|mut r| {
                        r.record.name = qname.clone();
                        r
                    })
                    .collect(),
            ),
            other => other,
        }
    }

    // ancestors get a node too so they block wildcards above them
    // a.b.home also creates b.home and home
    fn node_mut(&mut self, name: &str) -> &mut ZoneNode {
        for suffix in ancestors(name) {
            self.nodes.entry(suffix.into()).or_default();
        }
        self.nodes.entry(name.into()).or_default()
    }
}

// the last step decides if the chain continues
fn cname_target(records: &[ResolvedRecord]) -> Option<DomainName> {
    let record = &records.last()?.record;
    match &record.data {
        DnsRecordData::DomainName(target) if record.record_type == RecordType::CNAME => Some(target.clone()),
        _ => None,
    }
}

fn ancestors(name: &str) -> impl Iterator<Item = &str> {
    name.match_indices('.')
        .filter_map(|(i, _)| name.get(i + 1..))
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use reso_dns::ClassType;

    use super::*;

    fn name(s: &str) -> DomainName {
        DomainName::from_user(s).unwrap()
    }

    fn cname(owner: &str, target: &str) -> ResolvedRecord {
        ResolvedRecord {
            record: DnsRecord::new(
                name(owner),
                RecordType::CNAME,
                ClassType::IN,
                300,
                DnsRecordData::DomainName(name(target)),
            ),
        }
    }

    fn answer_names(lookup: LocalLookup) -> Vec<(String, RecordType)> {
        match lookup {
            LocalLookup::Answer(records) => records
                .iter()
                .map(|r| (r.record.name.to_string(), r.record.record_type))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn a(owner: &str, ip: [u8; 4]) -> ResolvedRecord {
        ResolvedRecord {
            record: DnsRecord::new(
                name(owner),
                RecordType::A,
                ClassType::IN,
                300,
                DnsRecordData::Ipv4(Ipv4Addr::from(ip)),
            ),
        }
    }

    fn answer_ip(lookup: LocalLookup) -> Option<(String, Ipv4Addr)> {
        match lookup {
            LocalLookup::Answer(records) => match &records[0].record.data {
                DnsRecordData::Ipv4(ip) => Some((records[0].record.name.to_string(), *ip)),
                _ => None,
            },
            _ => None,
        }
    }

    #[test]
    fn test_exact_beats_wildcard() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(a("home", [10, 0, 0, 1]));
        zone.insert_exact(a("nas.home", [10, 0, 0, 2]));

        assert_eq!(
            answer_ip(zone.lookup(&name("nas.home"), RecordType::A)),
            Some(("nas.home".into(), Ipv4Addr::new(10, 0, 0, 2)))
        );
    }

    #[test]
    fn test_wildcard_synthesizes_qname() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(a("home", [10, 0, 0, 1]));

        assert_eq!(
            answer_ip(zone.lookup(&name("x.home"), RecordType::A)),
            Some(("x.home".into(), Ipv4Addr::new(10, 0, 0, 1)))
        );
        // Multiple labels match as long as nothing in between exists.
        assert_eq!(
            answer_ip(zone.lookup(&name("a.b.home"), RecordType::A)),
            Some(("a.b.home".into(), Ipv4Addr::new(10, 0, 0, 1)))
        );
        // The wildcard does not match its parent.
        assert!(matches!(
            zone.lookup(&name("home"), RecordType::A),
            LocalLookup::NotFound
        ));
    }

    #[test]
    fn test_existing_name_blocks_wildcard_below_it() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(a("home", [10, 0, 0, 1]));
        zone.insert_exact(a("sub.home", [10, 0, 0, 2]));

        // Closest encloser is sub.home, which has no wildcard.
        assert!(matches!(
            zone.lookup(&name("x.sub.home"), RecordType::A),
            LocalLookup::NotFound
        ));
    }

    #[test]
    fn test_empty_non_terminal_blocks_wildcard() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(a("home", [10, 0, 0, 1]));
        zone.insert_exact(a("a.b.home", [10, 0, 0, 3]));

        assert!(matches!(
            zone.lookup(&name("b.home"), RecordType::A),
            LocalLookup::NotFound
        ));
        assert!(matches!(
            zone.lookup(&name("x.b.home"), RecordType::A),
            LocalLookup::NotFound
        ));
        // Siblings of the empty non-terminal still match.
        assert!(answer_ip(zone.lookup(&name("c.home"), RecordType::A)).is_some());
    }

    #[test]
    fn test_closest_wildcard_wins() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(a("home", [10, 0, 0, 1]));
        zone.insert_wildcard(a("lab.home", [10, 0, 0, 4]));

        assert_eq!(
            answer_ip(zone.lookup(&name("x.lab.home"), RecordType::A)),
            Some(("x.lab.home".into(), Ipv4Addr::new(10, 0, 0, 4)))
        );
        assert_eq!(
            answer_ip(zone.lookup(&name("x.home"), RecordType::A)),
            Some(("x.home".into(), Ipv4Addr::new(10, 0, 0, 1)))
        );
    }

    #[test]
    fn test_existing_name_without_type_is_nodata() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(a("home", [10, 0, 0, 1]));
        zone.insert_exact(a("nas.home", [10, 0, 0, 2]));

        assert!(matches!(
            zone.lookup(&name("nas.home"), RecordType::AAAA),
            LocalLookup::NoData
        ));
    }

    #[test]
    fn test_unrelated_name_not_found() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(a("home", [10, 0, 0, 1]));

        assert!(matches!(
            zone.lookup(&name("example.com"), RecordType::A),
            LocalLookup::NotFound
        ));
        assert!(matches!(
            zone.lookup(&DomainName::root(), RecordType::A),
            LocalLookup::NotFound
        ));
    }

    #[test]
    fn test_cname_chain_followed() {
        let mut zone = LocalZone::default();
        zone.insert_exact(cname("alias.home", "mid.home"));
        zone.insert_exact(cname("mid.home", "host.home"));
        zone.insert_exact(a("host.home", [10, 0, 0, 1]));

        assert_eq!(
            answer_names(zone.lookup(&name("alias.home"), RecordType::A)),
            vec![
                ("alias.home".into(), RecordType::CNAME),
                ("mid.home".into(), RecordType::CNAME),
                ("host.home".into(), RecordType::A),
            ]
        );
    }

    #[test]
    fn test_cname_query_not_followed() {
        let mut zone = LocalZone::default();
        zone.insert_exact(cname("alias.home", "host.home"));
        zone.insert_exact(a("host.home", [10, 0, 0, 1]));

        assert_eq!(
            answer_names(zone.lookup(&name("alias.home"), RecordType::CNAME)),
            vec![("alias.home".into(), RecordType::CNAME)]
        );
    }

    #[test]
    fn test_wildcard_cname_followed() {
        let mut zone = LocalZone::default();
        zone.insert_wildcard(cname("home", "host.home"));
        zone.insert_exact(a("host.home", [10, 0, 0, 1]));

        assert_eq!(
            answer_names(zone.lookup(&name("x.home"), RecordType::A)),
            vec![
                ("x.home".into(), RecordType::CNAME),
                ("host.home".into(), RecordType::A),
            ]
        );
    }

    #[test]
    fn test_cname_to_outside_target_ends_chain() {
        let mut zone = LocalZone::default();
        zone.insert_exact(cname("alias.home", "example.com"));

        assert_eq!(
            answer_names(zone.lookup(&name("alias.home"), RecordType::A)),
            vec![("alias.home".into(), RecordType::CNAME)]
        );
    }

    #[test]
    fn test_cname_loop_stops_at_depth_limit() {
        let mut zone = LocalZone::default();
        zone.insert_exact(cname("a.home", "b.home"));
        zone.insert_exact(cname("b.home", "a.home"));

        let answers = answer_names(zone.lookup(&name("a.home"), RecordType::A));
        assert_eq!(answers.len(), MAX_CNAME_CHAIN + 1);
    }
}
