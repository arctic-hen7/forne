const RESPONSES = ["y", "n"];

fn get_weight(data, difficult) {
   print(get_seconds_since_epoch());
   return data.weight;
}
fn adjust_card(res, data, difficult) {
   if res == "y" {
       data.weight -= 0.5;
   } else {
       data.weight += 0.5;
   }

   return [data, false];
}
fn get_default_metadata() {
   return #{ weight: 1.0 };
}
