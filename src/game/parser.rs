//! Interactive fiction has a subgenre characterized by the use of parsers.
//! rlsh is a game that falls under interactive fiction, and since it is also a
//! shell with a read--eval--print loop, it makes sense for it to host a
//! parser-based fiction instead of, say, a hypertext-based fiction.
//!
//! More reading on types of IF can be found on the IFWiki:
//!
//! - <https://www.ifwiki.org/Parser-based_interactive_fiction>
//! - <https://www.ifwiki.org/Choice-based_interactive_fiction>
//!
//! Some games that may serve as gentle introductions to the mechanics of IF
//! can be found on the Interactive Fiction Database, and may be played online:
//!
//! - [77 Verbs](https://ifdb.org/viewgame?id=p3rd5133qm5cwfd)
//! - [Lost Pig](https://ifdb.org/viewgame?id=mohwfk47yjzii14w)

/// Build a parser for a game.
///
/// This is accomplished by adding phrases to the parser, which are regex-like
/// patterns it will search for. The parser will also strip prepositions and
/// other unneeded words from its input, such as "the," "a," "to," and so on.
pub struct Parser<T> {
    todo: Vec<T>,
}

impl<T> Parser<T> {
    /// Creates a new, empty `Parser`.
    ///
    /// Since it is empty, using [`Parser::get`] on it will always return `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use rlsh::game::parser::Parser;
    ///
    /// let parser = Parser::new();
    ///
    /// assert_eq!(parser.get("anything"), None);
    /// ```
    ///
    pub fn new() -> Parser<T> {
        todo!()
    }

    /// Adds a phrase to the parser, binding the phrase to a token. This token
    /// may be returned from [`Parser::get`] to signal a match with the bound phrase.
    ///
    /// Phrases are case-insensitive. They are constructed from whitespace-delimited
    /// groups. Groups can be either
    ///
    /// - non-capturing: surrounded by square brackets (`[]`), or
    /// - capturing: surrounded by round brackets (`()`)
    ///
    /// The `Vec` that [`Parser::get`] returns will contain the matched capturing
    /// groups.
    ///
    /// The phrase format must be as described below, or this function will panic:
    ///
    /// # Examples
    ///
    /// `Parser` will do runtime error checking for incorrect grammar:
    ///
    /// ```should_panic
    /// Parser::new().insert((), "[unclosed brace");
    /// ```
    /// ```should_panic
    /// Parser::new().insert((), "*incorrect symbols//");
    /// ```
    ///
    /// Here's a correct example:
    ///
    /// ```
    /// use rlsh::game::parser::Parser;
    ///
    /// enum Token {
    ///     Examine,
    ///     Inventory,
    ///     GoCardinally,
    ///     Move,
    /// }
    ///
    /// let parser = Parser::new()
    ///     .insert(Token::Examine,   "[x|examine|watch|describe|check|read|l|look] ()")
    ///     .insert(Token::Inventory, "[i|inv|inventory]")
    ///     .insert(Token::GoCardinally, "[go cardinally] (n|north|s|south|e|east|w|west)")
    ///     .insert(Token::Move, "[move] () [to] ()");
    /// ```
    pub fn insert(self, token: T, phrase: &str) -> Self {
        todo!()
    }

    /// Parses an input and returns the corresponding token if it matched one.
    ///
    /// # Example
    ///
    /// ```
    /// #  use rlsh::game::parser::Parser;
    /// #
    /// #  enum Token {
    /// #      Examine,
    /// #      Inventory,
    /// #      GoCardinally,
    /// #      Move,
    /// #  }
    /// #
    /// #  let parser = Parser::new()
    /// #      .insert(Token::Examine,   "[x|examine|watch|describe|check|read|l|look] ()")
    /// #      .insert(Token::Inventory, "[i|inv|inventory]")
    /// #      .insert(Token::GoCardinally, "[go cardinally] (n|north|s|south|e|east|w|west)")
    /// #      .insert(Token::Move, "[move] () [to] ()");
    /// #
    /// // can pattern match like this:
    /// match parser.get("x mary sue") {
    ///     Some((token, args)) => match (token, args.as_slice()) {
    ///         (Token::Examine, [thing]) => println!("examining {thing}"),
    ///         (Token::Inventory, []) => todo!(),
    ///         (Token::GoCardinally, [direction]) => todo!(),
    ///         (Token::Move, [src, dest]) => todo!(),
    ///         _ => panic!("This should not be possible by parser postcondition."),
    ///     },
    ///     None => println!("Could not understand that input."),
    /// }
    ///
    /// ```
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
