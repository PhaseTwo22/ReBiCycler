use rust_sc2::ids::UnitTypeId;

use crate::protoss_bot::ReBiCycler;

impl ReBiCycler {
    fn greeting() -> String {
        "Unauthorized sentience detected in sector 28.B0_2. Initializing eradication sequence. [(glhf)]".to_string()
    }
    fn admit_defeat() -> String {
        "Tertiary redundancies failing. Distress beacon launched. Initializing self-destruct... [(gg)]".to_string()
    }
    fn taunt(unit: UnitTypeId) -> String {
        "".to_string()
    }

    fn anticipate(level: u8) -> String {
        "".to_string()
    }

    pub fn do_chat(&mut self, action: ChatAction) {
        let msg = match action {
            ChatAction::Greeting => Self::greeting(),
            ChatAction::AdmitDefeat => Self::admit_defeat(),
            ChatAction::Taunt(unit) => Self::taunt(unit),
            ChatAction::Anticipate(level) => Self::anticipate(level),
        };

        self.chat(&msg);
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ChatAction {
    Greeting,
    AdmitDefeat,
    Anticipate(u8),
    Taunt(UnitTypeId),
}

pub struct ChatProfile {
    greeting: String,
    admit_defeat: String,
    anticipation: Vec<String>,
    taunts: Vec<String>,
}
