use crate::queue::Member;

/// Хто зараз у кабінці.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Occupant {
    /// Зайняли кнопкою без черги, або першим у черзі був анонім.
    Button,
    User(String),
}

impl From<&Member> for Occupant {
    fn from(m: &Member) -> Self {
        match m {
            Member::Button => Occupant::Button,
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
    let user = match &w.occupant {
        Occupant::Button => "Зареєстровано кнопкою".into(),
        Occupant::User(name) => name.clone(),
    };
    format!("Температура: {}\nЧас: {}\nКористувач: {}", w.level, w.time, user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_texts() {
        assert_eq!(water_info(None), "Температура не встановлена");
        let w = WaterTemperature { level: 2, occupant: Occupant::User("@vova".into()), time: "14:23:11".into() };
        assert_eq!(water_info(Some(&w)), "Температура: 2\nЧас: 14:23:11\nКористувач: @vova");
        let w = WaterTemperature { occupant: Occupant::Button, ..w };
        assert!(water_info(Some(&w)).ends_with("Користувач: Зареєстровано кнопкою"));
    }

    #[test]
    fn occupant_from_member() {
        assert_eq!(Occupant::from(&Member::Button), Occupant::Button);
        assert_eq!(Occupant::from(&Member::user(3, "")), Occupant::User("3".into()));
    }
}
