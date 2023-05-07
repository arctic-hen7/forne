const RESPONSES = ["y", "n"];

fn get_weight(card) {
   return card.method_data.weight;
}
fn adjust_card(res, card) {
   if res == "y" {
       card.method_data.weight -= 0.5;
   } else {
       card.method_data.weight += 0.5;
   }

   return [card, false];
}
fn get_default_metadata() {
   return #{ weight: 1.0 };
}
