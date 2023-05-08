use crate::{
    set::{CardType, Set},
    SlimCard,
};

impl Set {
    /// Lists all the terms in the set of the given type, returning them as pairs of questions and answers.
    ///
    /// *Note: it is deliberately impossible to return card metadata through the traditional interface, and one should
    /// independently process that if this is required. The generical philosophy of Forne is not to interact with
    /// the method-specific metadata whenever possible, however.*
    pub fn list(&self, ty: CardType) -> Vec<SlimCard> {
        self.cards
            .values()
            .filter(|card| {
                ty == CardType::All
                    || (ty == CardType::Difficult && card.difficult)
                    || (ty == CardType::Starred && card.starred)
            })
            .map(|card| SlimCard {
                question: card.question.to_string(),
                answer: card.answer.to_string(),
                difficult: card.difficult,
                starred: card.starred,
            })
            .collect::<_>()
    }
}
