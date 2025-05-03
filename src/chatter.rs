use rust_sc2::ids::UnitTypeId;

use crate::protoss_bot::ReBiCycler;

pub struct ChatController {
    sent_all_chats: Vec<ChatAction>,
    profile: Box<dyn ChatProfile>,
}

impl Default for ChatController {
    fn default() -> Self {
        Self {
            sent_all_chats: Vec::new(),
            profile: Box::new(RobotSanitizer),
        }
    }
}

impl ChatController {
    fn add_sent(&mut self, chat: ChatAction) {
        self.sent_all_chats.push(chat);
    }

    fn has_sent(&self, chat: ChatAction) -> bool {
        self.sent_all_chats.contains(&chat)
    }
}
impl ReBiCycler {
    pub fn do_chat(&mut self, action: ChatAction) {
        if self.chat_controller.has_sent(action) {
            return;
        }

        let msg = match action {
            ChatAction::Greeting => self.chat_controller.profile.greeting(),
            ChatAction::AdmitDefeat => self.chat_controller.profile.admit_defeat(),
            ChatAction::Taunt(unit) => self.chat_controller.profile.taunt(unit),
            ChatAction::Anticipate(level) => self.chat_controller.profile.anticipate(level),
        };
        self.chat_controller.add_sent(action);
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

trait ChatProfile {
    fn greeting(&self) -> String;
    fn admit_defeat(&self) -> String;
    fn taunt(&self, unit: UnitTypeId) -> String;
    fn anticipate(&self, level: u8) -> String;
}

struct RobotSanitizer;
impl ChatProfile for RobotSanitizer {
    fn greeting(&self) -> String {
        "Unauthorized sentience detected in sector 28.B0_2. Initializing eradication sequence. [(glhf)]".to_string()
    }
    fn admit_defeat(&self) -> String {
        "Tertiary redundancies failing. Distress beacon launched. Initializing self-destruct... [(gg)]".to_string()
    }
    fn taunt(&self, unit: UnitTypeId) -> String {
        format!("Initiating purification protocol: {unit:?} detected.")
    }

    fn anticipate(&self, level: u8) -> String {
        match level {
            0 => "Sentience has failed to self-eradicate. Force Authorization incremented: [Low]",
            1 => "Initializing combatant catapult. ",
            2 => "Force Authorization incremented: [Moderate]",
            3 => "Enhancing sanctifiers, 69%.",
            4 => "Force Authorization incremented: [Maximum] | Our cannons shall SING!",
            _ => "",
        }
        .to_string()
    }
}
