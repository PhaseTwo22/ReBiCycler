use crate::protoss_bot::ReBiCycler;

impl ReBiCycler{
     pub fn greeting(&mut self){
         let msg = "Unauthorized sentience detected in sector 28.B0_2. Initializing eradication sequence. [(glhf)]"
         self.chat(msg);
} 
     pub fn admit_defeat(&mut self) {
          let msg = "Tertiary redundancies failing. Distress beacon launched. Initializing self-destruct... [(gg)]"

          self.chat(msg);
}
}