use crate::ini::{self, Entry, Section};
use crate::messages;

#[derive(Clone, Debug, Default)]
pub struct Reservation {
    pub driver: String,
    pub car: String,
    pub skin: String,
    /// SteamID64 del giocatore: senza GUID AC non fa entrare nello slot prenotato.
    pub guid: String,
}

fn entry_value(section: &Section, key: &str) -> String {
    section
        .entries
        .iter()
        .find(|entry| entry.key.eq_ignore_ascii_case(key))
        .map(|entry| entry.value.trim().to_string())
        .unwrap_or_default()
}

fn is_free(section: &Section) -> bool {
    entry_value(section, "GUID").is_empty() && entry_value(section, "DRIVERNAME").is_empty()
}

fn set_entry(section: &mut Section, key: &str, value: &str) {
    if let Some(entry) = section
        .entries
        .iter_mut()
        .find(|entry| entry.key.eq_ignore_ascii_case(key))
    {
        entry.value = value.to_string();
    } else {
        section.entries.push(Entry {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
}

/// Numero di slot che nessuno ha ancora occupato (GUID e DRIVERNAME vuoti).
pub fn free_slot_count(text: &str) -> usize {
    ini::parse(text)
        .sections
        .iter()
        .filter(|section| section.name.to_uppercase().starts_with("CAR_"))
        .filter(|section| is_free(section))
        .count()
}

fn sanitize(value: &str, label: &str, max: usize) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed
        .chars()
        .any(|c| c.is_control() || "[];=".contains(c))
    {
        return Err(messages::tr(
            &format!("{} contiene caratteri non ammessi", label),
            &format!("{} contains forbidden characters", label),
        ));
    }
    if trimmed.chars().count() > max {
        return Err(messages::tr(
            &format!("{} troppo lungo (max {})", label, max),
            &format!("{} is too long (max {})", label, max),
        ));
    }
    Ok(trimmed.to_string())
}

/// Scrive la prenotazione in uno slot libero, senza toccare formattazione e
/// chiavi non gestite. Restituisce il testo aggiornato e l'indice dello slot.
pub fn apply_reservation(
    text: &str,
    cap: usize,
    reservation: &Reservation,
) -> Result<(String, usize), String> {
    let driver = sanitize(&reservation.driver, "nome pilota", 40)?;
    let car = sanitize(&reservation.car, "auto", 64)?;
    let skin = sanitize(&reservation.skin, "livrea", 64)?;
    if driver.is_empty() || car.is_empty() {
        return Err(messages::tr(
            "nome pilota e auto sono obbligatori",
            "driver name and car are required",
        ));
    }

    let mut sections = ini::parse(text).sections;
    let car_positions: Vec<usize> = sections
        .iter()
        .enumerate()
        .filter(|(_, section)| section.name.to_uppercase().starts_with("CAR_"))
        .map(|(index, _)| index)
        .collect();

    let already = car_positions.iter().copied().find(|index| {
        let section = &sections[*index];
        entry_value(section, "DRIVERNAME") == driver
            || (!reservation.guid.trim().is_empty()
                && entry_value(section, "GUID").eq_ignore_ascii_case(reservation.guid.trim()))
    });
    let same_car = car_positions
        .iter()
        .copied()
        .find(|index| is_free(&sections[*index]) && entry_value(&sections[*index], "MODEL") == car);
    let any_free = car_positions
        .iter()
        .copied()
        .find(|index| is_free(&sections[*index]));

    let target = match already.or(same_car).or(any_free) {
        Some(index) => index,
        None => {
            if car_positions.len() >= cap && cap > 0 {
                return Err(messages::tr(
                    "nessuno slot libero: MAX_CLIENTS raggiunto",
                    "no free slot: MAX_CLIENTS reached",
                ));
            }
            let next = car_positions
                .iter()
                .filter_map(|index| {
                    sections[*index]
                        .name
                        .trim_start_matches(|c: char| !c.is_ascii_digit())
                        .parse::<usize>()
                        .ok()
                })
                .max()
                .map(|value| value + 1)
                .unwrap_or(car_positions.len());
            sections.push(Section {
                name: format!("CAR_{}", next),
                entries: Vec::new(),
            });
            sections.len() - 1
        }
    };

    let section = &mut sections[target];
    set_entry(section, "DRIVERNAME", &driver);
    set_entry(section, "MODEL", &car);
    set_entry(section, "SKIN", &skin);
    set_entry(section, "GUID", reservation.guid.trim());

    let position = car_positions
        .iter()
        .position(|index| *index == target)
        .unwrap_or(car_positions.len());

    Ok((ini::apply(text, &sections), position))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "; entry list\r\n[CAR_0]\r\nMODEL=bmw_z4_gt3\r\nSKIN=one\r\nDRIVERNAME=\r\nGUID=\r\n\r\n[CAR_1]\r\nMODEL=ferrari_458\r\nSKIN=two\r\nDRIVERNAME=Mario\r\nGUID=abc\r\n";

    fn reservation(driver: &str, car: &str, skin: &str) -> Reservation {
        Reservation {
            driver: driver.into(),
            car: car.into(),
            skin: skin.into(),
            guid: String::new(),
        }
    }

    #[test]
    fn free_slots_counted() {
        assert_eq!(free_slot_count(BASE), 1);
    }

    #[test]
    fn reserves_existing_free_slot() {
        let (out, slot) =
            apply_reservation(BASE, 4, &reservation("Luca", "bmw_z4_gt3", "one")).unwrap();
        assert_eq!(slot, 0);
        assert!(out.contains("; entry list"));
        assert!(out.contains("DRIVERNAME=Luca"));
        assert!(out.contains("SKIN=one"));
        assert_eq!(out.matches("[CAR_").count(), 2);
    }

    #[test]
    fn adds_slot_when_cap_allows() {
        let full = BASE.replace("DRIVERNAME=\r\nGUID=", "DRIVERNAME=Giulia\r\nGUID=xyz");
        let (out, slot) =
            apply_reservation(&full, 4, &reservation("Luca", "bmw_z4_gt3", "one")).unwrap();
        assert_eq!(slot, 2);
        assert!(out.contains("[CAR_2]"));
        assert!(out.contains("DRIVERNAME=Luca"));
        assert!(out.contains("MODEL=bmw_z4_gt3"));
    }

    #[test]
    fn refuses_when_cap_reached() {
        let full = BASE.replace("DRIVERNAME=\r\nGUID=", "DRIVERNAME=Giulia\r\nGUID=xyz");
        assert!(apply_reservation(&full, 2, &reservation("Luca", "bmw_z4_gt3", "one")).is_err());
    }

    #[test]
    fn rejects_injected_values() {
        let attempt = Reservation {
            driver: "Mario\r\nGUID=hack".into(),
            car: "bmw_z4_gt3".into(),
            skin: "one".into(),
            guid: String::new(),
        };
        assert!(apply_reservation(BASE, 4, &attempt).is_err());
        let attempt = Reservation {
            driver: "Mario".into(),
            car: "x]=y".into(),
            skin: "one".into(),
            guid: String::new(),
        };
        assert!(apply_reservation(BASE, 4, &attempt).is_err());
    }

    #[test]
    fn second_reservation_updates_same_slot() {
        let (first, slot) =
            apply_reservation(BASE, 4, &reservation("Luca", "bmw_z4_gt3", "one")).unwrap();
        let (second, again) =
            apply_reservation(&first, 4, &reservation("Luca", "bmw_z4_gt3", "two")).unwrap();
        assert_eq!(slot, again);
        assert_eq!(second.matches("DRIVERNAME=Luca").count(), 1);
        assert!(second.contains("SKIN=two"));
    }

    #[test]
    fn same_steam_id_reuses_its_slot() {
        let mut first = reservation("Luca", "bmw_z4_gt3", "one");
        first.guid = "76561198000000001".into();
        let (text, slot) = apply_reservation(BASE, 4, &first).unwrap();

        // stesso SteamID, nome diverso: deve riusare lo slot, non crearne un altro
        let mut second = reservation("Luca2", "ferrari_458", "two");
        second.guid = "76561198000000001".into();
        let (text2, again) = apply_reservation(&text, 4, &second).unwrap();

        assert_eq!(slot, again);
        assert_eq!(text2.matches("GUID=76561198000000001").count(), 1);
        assert_eq!(text2.matches("DRIVERNAME=").count(), 2); // slot 0 + slot 1 (Mario)
    }

    #[test]
    fn new_slot_avoids_existing_numbers() {
        let gapped = "[CAR_1]\nMODEL=x\nSKIN=s\nDRIVERNAME=\nGUID=\n";
        let (out, slot) = apply_reservation(gapped, 4, &reservation("Luca", "y", "s")).unwrap();
        assert_eq!(slot, 0);
        assert!(out.contains("[CAR_1]"));
        assert!(!out.contains("[CAR_0]"));
        let full = gapped.replace("DRIVERNAME=\nGUID=", "DRIVERNAME=G\nGUID=z");
        let (out2, _) = apply_reservation(&full, 4, &reservation("Luca", "y", "s")).unwrap();
        assert!(out2.contains("[CAR_2]"));
    }
}
