pub struct Parser<T> {
    todo: Vec<T>,
}

///
impl<T> Parser<T> {
    pub fn new() -> Parser<T> {
        todo!()
    }

    pub fn insert(self, token: T, phrase: &str) -> Self {
        todo!()
    }

    pub fn get(&self, input: &str) -> Option<(T, Vec<String>)> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn query_for_phrases() {
        #[derive(Debug, PartialEq)]
        enum Token {
            Examine,
            Inventory,
            GoCardinally,
            Move,
        }

        #[rustfmt::skip]
        let parser = Parser::new()
            .insert(Token::Examine,   "[x|examine|watch|describe|check|read|l|look] ()")
            .insert(Token::Inventory, "[i|inv|inventory]")
            .insert(Token::GoCardinally, "[go cardinally] (n|north|s|south|e|east|w|west)")
            .insert(Token::Move, "[move] () [to] ()");

        // EXAMINE
        for word in "x|examine|watch|describe|check|read|l|look".split("|") {
            let query = String::from(word) + " at the book";
            assert_eq!(
                parser.get(&query),
                Some((Token::Examine, vec!["book".to_string()]))
            );
        }

        assert_eq!(
            parser.get("x mary sue"),
            Some((Token::Examine, vec!["mary sue".to_string()]))
        );

        // INVENTORY
        for word in "i|inv|inventory".split("|") {
            assert_eq!(
                parser.get(word),
                Some((Token::Inventory, vec!["".to_string()]))
            );
        }

        // GO_CARDINALLY
        for word in "n|north|s|south|e|east|w|west".split("|") {
            let query = String::from("go cardinally to ") + word;
            assert_eq!(
                parser.get(&query),
                Some((Token::GoCardinally, vec![word.to_string()]))
            );
        }

        assert_eq!(parser.get("go cardinally to heaven"), None);
        assert_eq!(parser.get("go cardinally toward free lunch"), None);

        // MOVE
        assert_eq!(
            parser.get("move the book to the bookshelf"),
            Some((
                Token::Move,
                vec!["book".to_string(), "bookshelf".to_string()]
            ))
        );

        // MISC FALIURES
        assert_eq!(parser.get("explode the book"), None);
        assert_eq!(parser.get("yap about the book"), None);
        assert_eq!(parser.get("take the book on a stroll"), None);
    }
}
