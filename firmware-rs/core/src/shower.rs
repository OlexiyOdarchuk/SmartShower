use crate::queue::Member;

/// Хто зараз у кабінці.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Occupant {
    Ticket(u16),
    User(String),
    /// Зайшли поза чергою.
    WalkIn,
}

impl Occupant {
    pub fn display_name(&self) -> String {
        match self {
            Occupant::Ticket(n) => Member::Ticket(*n).display_name(),
            Occupant::User(name) => name.clone(),
            Occupant::WalkIn => "Без черги".into(),
        }
    }
}

impl From<&Member> for Occupant {
    fn from(m: &Member) -> Self {
        match m {
            Member::Ticket(n) => Occupant::Ticket(*n),
            user => Occupant::User(user.display_name()),
        }
    }
}

/// Остання вибрана температура води (1..=4) і хто її вибрав.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaterTemperature {
    pub level: u8,
    pub occupant: Occupant,
    /// Локальний час «ГГ:ХХ:СС».
    pub time: String,
}

pub fn water_info(water: Option<&WaterTemperature>) -> String {
    let Some(w) = water else {
        return "Температура не встановлена".into();
    };
    format!("Температура: {}\nЧас: {}\nКористувач: {}", w.level, w.time, w.occupant.display_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_texts() {
        assert_eq!(water_info(None), "Температура не встановлена");
        let w = WaterTemperature { level: 2, occupant: Occupant::User("@vova".into()), time: "14:23:11".into() };
        assert_eq!(water_info(Some(&w)), "Температура: 2\nЧас: 14:23:11\nКористувач: @vova");
        let w = WaterTemperature { occupant: Occupant::Ticket(7), ..w };
        assert!(water_info(Some(&w)).ends_with("Користувач: Талон №7"));
    }

    #[test]
    fn occupant_from_member() {
        assert_eq!(Occupant::from(&Member::Ticket(3)), Occupant::Ticket(3));
        assert_eq!(Occupant::from(&Member::user(3, "")), Occupant::User("3".into()));
        assert_eq!(Occupant::WalkIn.display_name(), "Без черги");
    }
}
