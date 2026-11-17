use crate::ops::softmax;

pub trait LogitConstraint {
    fn apply_mask(&mut self, logits: &mut [f32], vocab_fn: &dyn Fn(u32) -> String);
    fn accept_token(&mut self, token_id: u32, token_str: &str);
    fn is_done(&self) -> bool {
        false
    }
    fn notify_remaining_steps(&mut self, _remaining: usize) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparseChannel {
    Unconstrained,
    Pir,
    Zettel,
    Clarify,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenClass(pub u16);

impl TokenClass {
    pub const NONE: u16 = 0;
    pub const PIR_OPEN: u16 = 1 << 0; // 353 "<pir>"
    pub const PIR_CLOSE: u16 = 1 << 1; // 354 "</pir>"
    pub const OP_F: u16 = 1 << 2; // 269 "op_f", "f"
    pub const OP_STMT: u16 = 1 << 3; // 264..=273 ("v", "=", "p", "?", "w", "r", "c", "m")
    pub const LPAREN: u16 = 1 << 4; // 261 "("
    pub const RPAREN: u16 = 1 << 5; // 262 ")"
    pub const LBRACKET: u16 = 1 << 6; // 259 "["
    pub const RBRACKET: u16 = 1 << 7; // 260 "]"
    pub const IDENT: u16 = 1 << 8; // valid variable/function identifiers
    pub const LITERAL: u16 = 1 << 9; // numeric literals, quoted strings
    pub const OPERATOR: u16 = 1 << 10; // "+", "-", "*", "/", "<", ">", "="
    pub const BUILTIN: u16 = 1 << 11; // "@rd", "@wt", "@x*"
    pub const WHITESPACE: u16 = 1 << 12; // " ", "\n", "\t"
    pub const TALK_PROSE: u16 = 1 << 13; // printable natural language prose (> 400 or ASCII >= 0x20)
    pub const CONTROL: u16 = 1 << 14; // <ctrl_*>, special control tags
    pub const EOS: u16 = 1 << 15; // token 0, 2, "</pir>"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdaState {
    ExpectPirOpen,
    ExpectFunctionIdent,
    InFunctionIdent,
    ExpectParamsOpen,
    InParams,
    ExpectBlockOpen,
    StatementStart,
    VarDeclName,
    InVarDeclName,
    ExpectRhs,
    InExpression,
    InBareRhs,
    FunctionClosed,
    Done,
}

pub struct PirPdaConstraint {
    pub active: bool,
    pub channel: SparseChannel,
    pub state: PdaState,
    pub block_depth: usize,
    pub paren_depth: usize,
    pub tokens_emitted: usize,
    pub token_classes: Vec<TokenClass>,
    pub force_close_mode: bool,
    pub pending_block_after_expr: bool,
    pub allow_else_block: bool,
    pub consecutive_idents: usize,
    pub in_string: bool,
}

impl PirPdaConstraint {
    pub fn new(active: bool, vocab_size: usize, vocab_fn: &dyn Fn(u32) -> String) -> Self {
        Self::from_prompt(
            if active {
                SparseChannel::Pir
            } else {
                SparseChannel::Unconstrained
            },
            vocab_size,
            vocab_fn,
            "",
        )
    }

    pub fn new_simple(active: bool) -> Self {
        let vocab_fn = |id: u32| format!("tok_{}", id);
        Self::from_prompt(
            if active {
                SparseChannel::Pir
            } else {
                SparseChannel::Unconstrained
            },
            10,
            &vocab_fn,
            "",
        )
    }

    pub fn new_with_state(
        active: bool,
        vocab_size: usize,
        vocab_fn: &dyn Fn(u32) -> String,
        state: PdaState,
        block_depth: usize,
    ) -> Self {
        let mut c = Self::from_prompt(SparseChannel::Pir, vocab_size, vocab_fn, "");
        c.active = active;
        c.state = state;
        c.block_depth = block_depth;
        c
    }

    pub fn new_unclassified(
        channel: SparseChannel,
        state: PdaState,
        block_depth: usize,
        token_classes: Vec<TokenClass>,
    ) -> Self {
        Self {
            active: channel != SparseChannel::Unconstrained,
            channel,
            state,
            block_depth,
            paren_depth: 0,
            tokens_emitted: 0,
            token_classes,
            force_close_mode: false,
            pending_block_after_expr: false,
            in_string: false,
            allow_else_block: false,
            consecutive_idents: 0,
        }
    }

    fn strip_error_frames(text: &str) -> String {
        let mut out = String::new();
        let mut in_err = false;
        let lines = text.lines();
        for line in lines {
            if line.contains("<error>") {
                in_err = true;
            }
            if !in_err {
                out.push_str(line);
                out.push('\n');
            }
            if line.contains("</error>") {
                in_err = false;
            }
        }
        if out.is_empty() { text.to_string() } else { out }
    }

    // A ~100M ternary model will hallucinate syntax if left unconstrained.
    // This PDA masks logits at every step so it can only emit structurally valid pIR tokens.
    pub fn from_prompt(
        channel: SparseChannel,
        vocab_size: usize,
        vocab_fn: &dyn Fn(u32) -> String,
        prompt: &str,
    ) -> Self {
        let token_classes = Self::classify_vocab(vocab_size, vocab_fn);
        let mut state = match channel {
            SparseChannel::Pir => PdaState::ExpectPirOpen,
            SparseChannel::Unconstrained => PdaState::ExpectPirOpen,
            SparseChannel::Clarify => PdaState::InExpression,
            SparseChannel::Zettel => PdaState::StatementStart,
        };
        let mut block_depth = 0;

        let trimmed = prompt.trim();
        if channel == SparseChannel::Pir {
            if let Some(pos) = trimmed.rfind("<pir>") {
                let after_pir = trimmed[pos + 5..].trim();
                let opens = after_pir.chars().filter(|&c| c == '[' || c == '{').count();
                let closes = after_pir.chars().filter(|&c| c == ']' || c == '}').count();
                block_depth = opens.saturating_sub(closes);

                if after_pir.is_empty() {
                    state = PdaState::StatementStart;
                    block_depth = 0;
                } else if block_depth > 0 {
                    state = PdaState::StatementStart;
                } else if after_pir.starts_with('f') || after_pir.starts_with("fn") {
                    state = PdaState::ExpectParamsOpen;
                } else {
                    state = PdaState::StatementStart;
                    block_depth = 0;
                }
            } else {
                let cleaned = Self::strip_error_frames(trimmed);
                let cleaned_trim = cleaned.trim();
                let opens = cleaned_trim.chars().filter(|&c| c == '[' || c == '{').count();
                let closes = cleaned_trim.chars().filter(|&c| c == ']' || c == '}').count();
                let unmatched = opens.saturating_sub(closes);
                if unmatched > 0 {
                    state = PdaState::StatementStart;
                    block_depth = unmatched;
                } else if (cleaned_trim.starts_with("f ") || cleaned_trim.starts_with("fn "))
                    && opens == 0 {
                        state = PdaState::ExpectParamsOpen;
                    }
            }
        }
        Self {
            active: channel != SparseChannel::Unconstrained,
            channel,
            state,
            block_depth,
            paren_depth: 0,
            tokens_emitted: 0,
            token_classes,
            force_close_mode: false,
            pending_block_after_expr: false,
            in_string: false,
            allow_else_block: false,
            consecutive_idents: 0,
        }
    }

    pub fn classify_vocab_static(
        vocab_size: usize,
        vocab_fn: &dyn Fn(u32) -> String,
    ) -> Vec<TokenClass> {
        Self::classify_vocab(vocab_size, vocab_fn)
    }

    fn classify_vocab(vocab_size: usize, vocab_fn: &dyn Fn(u32) -> String) -> Vec<TokenClass> {
        let mut classes = Vec::with_capacity(vocab_size);
        let stopwords = [
            "writes", "missing", "clean", "statemen", "Visiting", "Quer", "comp", "ad", "acy",
            "be", "pat", "ompa", "stati", "eval",
        ];

        for id in 0..vocab_size {
            let tok = vocab_fn(id as u32);
            let mut c = 0u16;
            let is_eos = matches!(id, 0 | 2 | 352 | 354 | 356 | 362)
                || tok == "</pir>"
                || tok == "</talk>"
                || tok == "</fim>"
                || tok == "</zettel>";
            if is_eos {
                c |= TokenClass::EOS;
            }
            if id == 354 || tok.contains("</pir>") {
                c |= TokenClass::PIR_CLOSE;
            }

            let is_control = !is_eos
                && ((351..=400).contains(&id)
                    || tok.starts_with("<ctrl_")
                    || tok.starts_with("<reserved_")
                    || (tok.starts_with('<') && tok.ends_with('>') && !tok.contains("<pir>"))
                    || (tok.len() == 1
                        && (tok.as_bytes()[0] < 32 && !matches!(tok.as_bytes()[0], 9 | 10 | 13))));
            if is_control {
                c |= TokenClass::CONTROL;
            }

            let trimmed_val = tok.trim_matches(|ch: char| ch == ' ' || ch == '\t');
            if id == 353 || tok.contains("<pir>") {
                c |= TokenClass::PIR_OPEN;
            }
            if id == 269 || trimmed_val == "f" || tok == "op_f" {
                c |= TokenClass::OP_F;
            }
            if (264..=268).contains(&id)
                || matches!(id, 270 | 271 | 273)
                || matches!(trimmed_val, "v" | "r" | "p" | "?" | "w" | "=" | "c" | "m")
            {
                c |= TokenClass::OP_STMT;
            }
            if id == 259 || trimmed_val == "[" {
                c |= TokenClass::LBRACKET;
            }
            if id == 260 || trimmed_val == "]" {
                c |= TokenClass::RBRACKET;
            }
            if id == 261 || trimmed_val == "(" {
                c |= TokenClass::LPAREN;
            }
            if id == 262 || trimmed_val == ")" {
                c |= TokenClass::RPAREN;
            }
            if (287..=299).contains(&id)
                || matches!(
                    trimmed_val,
                    "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | "<=" | ">" | ">="
                )
            {
                c |= TokenClass::OPERATOR;
            }
            if (300..=350).contains(&id)
                || matches!(
                    trimmed_val,
                    "@rd"
                        | "@wt"
                        | "@ls"
                        | "@ex"
                        | "@if"
                        | "@id"
                        | "@mk"
                        | "@xh"
                        | "@xs"
                        | "@xj"
                        | "@xt"
                )
                || trimmed_val.starts_with("@x")
            {
                c |= TokenClass::BUILTIN;
            }
            if !tok.is_empty() && tok.chars().all(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r')) {
                c |= TokenClass::WHITESPACE;
            }
            if (!trimmed_val.is_empty()
                && !trimmed_val.starts_with('.')
                && !trimmed_val.ends_with('.')
                && trimmed_val.chars().any(|ch| ch.is_ascii_digit())
                && trimmed_val
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || ch == '.'))
                || (trimmed_val.starts_with('-')
                    && trimmed_val.len() > 1
                    && trimmed_val[1..].chars().all(|ch| ch.is_ascii_digit()))
                || matches!(trimmed_val, "true" | "false")
                || trimmed_val.contains('"')
            {
                c |= TokenClass::LITERAL;
            }

            let ident_part = tok.strip_prefix(' ').unwrap_or(tok.as_str());
            let is_clean_ident = !ident_part.is_empty()
                && ident_part.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
                && ident_part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                && !matches!(
                    ident_part,
                    "true" | "false" | "f" | "v" | "r" | "p" | "w" | "c" | "m"
                )
                && !stopwords.contains(&ident_part)
                && id >= 401;

            if is_clean_ident {
                c |= TokenClass::IDENT;
            }

            let is_pinned_opcode = (256..=400).contains(&id);
            let is_printable = tok
                .chars()
                .all(|ch| ch.is_alphabetic() || ch.is_ascii_punctuation() || ch.is_whitespace());
            if is_printable && !is_pinned_opcode && !is_control {
                c |= TokenClass::TALK_PROSE;
            }

            classes.push(TokenClass(c));
        }
        classes
    }

    fn is_channel_mismatched_eos(channel: SparseChannel, token_id: usize) -> bool {
        match token_id {
            362 => channel != SparseChannel::Zettel, // </zettel>
            368 => channel != SparseChannel::Clarify, // </clarify>
            354 => !matches!(channel, SparseChannel::Pir | SparseChannel::Unconstrained), // </pir>
            _ => false,
        }
    }

    fn allowed_mask(&self) -> u16 {
        if self.state == PdaState::Done {
            return TokenClass::EOS;
        }
        if self.force_close_mode {
            if self.paren_depth > 0 {
                return TokenClass::RPAREN;
            } else if self.block_depth > 0 {
                return TokenClass::RBRACKET;
            }
        }
        match self.channel {
            SparseChannel::Clarify => {
                TokenClass::TALK_PROSE
                    | TokenClass::WHITESPACE
                    | TokenClass::LITERAL
                    | TokenClass::OPERATOR
                    | TokenClass::EOS
            }
            SparseChannel::Zettel => {
                TokenClass::IDENT
                    | TokenClass::LITERAL
                    | TokenClass::OPERATOR
                    | TokenClass::WHITESPACE
                    | TokenClass::TALK_PROSE
                    | TokenClass::EOS
            }
            SparseChannel::Pir | SparseChannel::Unconstrained => {
                if self.in_string {
                    return TokenClass::LITERAL
                        | TokenClass::IDENT
                        | TokenClass::OPERATOR
                        | TokenClass::WHITESPACE
                        | TokenClass::TALK_PROSE
                        | TokenClass::LPAREN
                        | TokenClass::RPAREN
                        | TokenClass::LBRACKET
                        | TokenClass::RBRACKET;
                }
                match self.state {
                PdaState::ExpectPirOpen => {
                    TokenClass::PIR_OPEN | TokenClass::OP_F | TokenClass::WHITESPACE
                }
                PdaState::ExpectFunctionIdent => TokenClass::IDENT | TokenClass::WHITESPACE,
                PdaState::InFunctionIdent => {
                    TokenClass::IDENT
                        | TokenClass::LITERAL
                        | TokenClass::LPAREN
                        | TokenClass::WHITESPACE
                }
                PdaState::ExpectParamsOpen => TokenClass::LPAREN | TokenClass::WHITESPACE,
                PdaState::InParams => {
                    TokenClass::IDENT
                        | TokenClass::LITERAL
                        | TokenClass::RPAREN
                        | TokenClass::WHITESPACE
                }
                PdaState::ExpectBlockOpen => TokenClass::LBRACKET | TokenClass::WHITESPACE,
                PdaState::StatementStart => {
                    let mut mask = TokenClass::OP_STMT
                        | TokenClass::OP_F
                        | TokenClass::BUILTIN
                        | TokenClass::WHITESPACE;
                    if self.block_depth > 0 {
                        mask |= TokenClass::RBRACKET | TokenClass::LPAREN;
                    }
                    if self.allow_else_block {
                        mask |= TokenClass::LBRACKET;
                    }
                    if self.block_depth == 0 {
                        mask |= TokenClass::PIR_CLOSE | TokenClass::EOS;
                    }
                    mask
                }
                PdaState::VarDeclName => TokenClass::IDENT | TokenClass::WHITESPACE,
                PdaState::InVarDeclName => {
                    TokenClass::IDENT | TokenClass::LITERAL | TokenClass::WHITESPACE
                }
                PdaState::ExpectRhs => {
                    TokenClass::LPAREN
                        | TokenClass::IDENT
                        | TokenClass::LITERAL
                        | TokenClass::BUILTIN
                        | TokenClass::WHITESPACE
                }
                PdaState::InExpression => {
                    if self.pending_block_after_expr && self.paren_depth == 0 {
                        TokenClass::LPAREN | TokenClass::WHITESPACE
                    } else {
                        let mut mask = TokenClass::LPAREN
                            | TokenClass::RPAREN
                            | TokenClass::OPERATOR
                            | TokenClass::BUILTIN
                            | TokenClass::WHITESPACE;
                        if self.consecutive_idents < 3 {
                            mask |= TokenClass::IDENT | TokenClass::LITERAL;
                        }
                        mask
                    }
                }
                PdaState::InBareRhs => {
                    let mut mask = TokenClass::OP_STMT
                        | TokenClass::OP_F
                        | TokenClass::BUILTIN
                        | TokenClass::RBRACKET
                        | TokenClass::WHITESPACE;
                    if self.consecutive_idents < 3 {
                        mask |= TokenClass::IDENT | TokenClass::LITERAL;
                    }
                    if self.allow_else_block || self.pending_block_after_expr {
                        mask |= TokenClass::LBRACKET;
                    }
                    if self.block_depth == 0 {
                        mask |= TokenClass::PIR_CLOSE | TokenClass::EOS;
                    }
                    mask
                }
                PdaState::FunctionClosed => {
                    TokenClass::OP_F
                        | TokenClass::PIR_CLOSE
                        | TokenClass::EOS
                        | TokenClass::WHITESPACE
                }
                PdaState::Done => TokenClass::EOS,
            }
            }
        }
    }
}
impl LogitConstraint for PirPdaConstraint {
    fn apply_mask(&mut self, logits: &mut [f32], _vocab_fn: &dyn Fn(u32) -> String) {
        if !self.active {
            return;
        }
        let mask = self.allowed_mask();
        let mut allowed_count = 0;
        for (i, _logit) in logits.iter().enumerate() {
            let cls = if i < self.token_classes.len() {
                self.token_classes[i].0
            } else {
                0
            };
            let mismatched = Self::is_channel_mismatched_eos(self.channel, i);
            if (cls & mask) != 0 && (cls & TokenClass::CONTROL) == 0 && !mismatched {
                allowed_count += 1;
            }
        }
        if allowed_count > 0 {
            for (i, logit) in logits.iter_mut().enumerate() {
                let cls = if i < self.token_classes.len() {
                    self.token_classes[i].0
                } else {
                    0
                };
                let mismatched = Self::is_channel_mismatched_eos(self.channel, i);
                if (cls & mask) == 0 || (cls & TokenClass::CONTROL) != 0 || mismatched {
                    *logit = -f32::INFINITY;
                }
            }
        }
    }

    fn accept_token(&mut self, _token_id: u32, token_str: &str) {
        if !self.active {
            return;
        }
        self.tokens_emitted += 1;

        if self.channel == SparseChannel::Clarify && self.tokens_emitted >= 64 {
            self.state = PdaState::Done;
            return;
        }
        if self.channel == SparseChannel::Zettel && self.tokens_emitted >= 25 {
            self.state = PdaState::Done;
            return;
        }

        let trimmed = token_str.trim();
        if token_str.contains('\n') {
            self.consecutive_idents = 0;
            self.in_string = false;
            if self.paren_depth == 0
                && matches!(
                    self.state,
                    PdaState::InVarDeclName
                        | PdaState::ExpectRhs
                        | PdaState::InExpression
                        | PdaState::InBareRhs
                )
            {
                self.state = if self.pending_block_after_expr {
                    PdaState::ExpectBlockOpen
                } else {
                    PdaState::StatementStart
                };
            }
        }
        if trimmed.is_empty() {
            if self.state == PdaState::InVarDeclName
                && (token_str.starts_with(' ') || token_str.starts_with('\t'))
            {
                self.state = PdaState::ExpectRhs;
            }
            return;
        }
        if token_str.chars().filter(|&c| c == '"').count() % 2 == 1 {
            self.in_string = !self.in_string;
        }
        if self.in_string {
            return;
        }
        if matches!(
            trimmed,
            "(" | ")" | "[" | "]" | "{" | "}" | "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "v" | "r" | "p" | "?" | "w" | "=" | "f"
        ) || trimmed.starts_with('@')
        {
            self.consecutive_idents = 0;
        } else if matches!(self.state, PdaState::InExpression | PdaState::InBareRhs)
            && (token_str.starts_with(' ') || token_str.starts_with('\t'))
        {
            self.consecutive_idents += 1;
        }

        match self.state {
            PdaState::ExpectPirOpen => {
                if trimmed.contains("<pir>") || trimmed == "f" || trimmed == "fn" {
                    self.state = if trimmed == "f" || trimmed == "fn" {
                        PdaState::ExpectFunctionIdent
                    } else {
                        PdaState::StatementStart
                    };
                }
            }
            PdaState::ExpectFunctionIdent => {
                if !trimmed.is_empty() {
                    self.state = PdaState::InFunctionIdent;
                }
            }
            PdaState::InFunctionIdent => {
                if trimmed == "(" {
                    self.paren_depth = 0;
                    self.state = PdaState::InParams;
                }
            }
            PdaState::ExpectParamsOpen => {
                if trimmed == "(" {
                    self.paren_depth = 0;
                    self.state = PdaState::InParams;
                }
            }
            PdaState::InParams => {
                if trimmed == ")" {
                    self.state = PdaState::ExpectBlockOpen;
                }
            }
            PdaState::ExpectBlockOpen => {
                if trimmed == "[" || trimmed == "{" {
                    self.block_depth += 1;
                    self.pending_block_after_expr = false;
                    self.state = PdaState::StatementStart;
                }
            }
            PdaState::StatementStart => {
                let was_else = self.allow_else_block;
                self.allow_else_block = false;

                if was_else && (trimmed == "[" || trimmed == "{") {
                    self.block_depth += 1;
                } else if trimmed == "f" || trimmed == "fn" {
                    self.state = PdaState::ExpectFunctionIdent;
                } else if trimmed == "v" || trimmed == "let" || trimmed == "=" {
                    self.state = PdaState::VarDeclName;
                } else if trimmed == "?" || trimmed == "w" {
                    self.pending_block_after_expr = true;
                    self.state = PdaState::InExpression;
                } else if matches!(trimmed, "r" | "p" | "return" | "m") || trimmed.starts_with('@')
                {
                    self.pending_block_after_expr = false;
                    self.state = PdaState::InExpression;
                } else if trimmed == "(" {
                    self.paren_depth = 1;
                    self.pending_block_after_expr = false;
                    self.state = PdaState::InExpression;
                } else if trimmed == "]" || trimmed == "}" {
                    self.block_depth = self.block_depth.saturating_sub(1);
                    if self.block_depth == 0 {
                        self.state = PdaState::FunctionClosed;
                    } else {
                        self.allow_else_block = true;
                    }
                } else if trimmed.contains("</pir>") {
                    self.state = PdaState::Done;
                }
            }
            PdaState::VarDeclName => {
                if !trimmed.is_empty() {
                    self.state = PdaState::InVarDeclName;
                }
            }
            PdaState::InVarDeclName => {
                if trimmed == "(" {
                    self.paren_depth = 1;
                    self.state = PdaState::InExpression;
                } else if token_str.starts_with(' ')
                    || token_str.starts_with('\t')
                    || token_str.starts_with('\n')
                {
                    if trimmed.is_empty() {
                        self.state = PdaState::ExpectRhs;
                    } else {
                        self.state = PdaState::InBareRhs;
                    }
                }
            }
            PdaState::ExpectRhs => {
                if trimmed == "(" {
                    self.paren_depth = 1;
                    self.state = PdaState::InExpression;
                } else if !trimmed.is_empty() {
                    self.consecutive_idents = 1;
                    self.state = PdaState::InBareRhs;
                }
            }
            PdaState::InExpression => {
                if trimmed == "(" {
                    self.paren_depth += 1;
                } else if trimmed == ")" {
                    if self.paren_depth > 0 {
                        self.paren_depth -= 1;
                    }
                    if self.paren_depth == 0 {
                        if self.pending_block_after_expr {
                            self.state = PdaState::ExpectBlockOpen;
                        } else {
                            self.consecutive_idents = 1;
                            self.state = PdaState::InBareRhs;
                        }
                    }
                } else if self.paren_depth == 0 {
                    if token_str.contains('\n') {
                        if self.pending_block_after_expr {
                            self.state = PdaState::ExpectBlockOpen;
                        } else {
                            self.state = PdaState::StatementStart;
                        }
                    } else if !matches!(
                        trimmed,
                        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">="
                    ) {
                        self.state = PdaState::InBareRhs;
                    }
                }
            }
            PdaState::InBareRhs => {
                if trimmed == "[" || trimmed == "{" {
                    self.block_depth += 1;
                    self.pending_block_after_expr = false;
                    self.allow_else_block = false;
                    self.state = PdaState::StatementStart;
                } else if token_str.contains('\n') {
                    if self.pending_block_after_expr {
                        self.state = PdaState::ExpectBlockOpen;
                    } else {
                        self.state = PdaState::StatementStart;
                    }
                } else if matches!(
                    trimmed,
                    "(" | "]" | "}" | "v" | "let" | "r" | "p" | "return" | "f" | "fn" | "?" | "w" | "="
                ) || trimmed.starts_with('@')
                {
                    self.state = PdaState::StatementStart;
                    self.accept_token(_token_id, token_str);
                } else if trimmed.contains("</pir>") {
                    self.state = PdaState::Done;
                }
            }
            PdaState::FunctionClosed => {
                if trimmed.contains("</pir>") {
                    self.state = PdaState::Done;
                } else if trimmed == "f" || trimmed == "fn" {
                    self.state = PdaState::ExpectFunctionIdent;
                }
            }
            PdaState::Done => {}
        }

        if self.channel == SparseChannel::Clarify
            && (_token_id == 368 || trimmed == "</clarify>" || self.tokens_emitted >= 96) {
                self.state = PdaState::Done;
            }
    }

    fn is_done(&self) -> bool {
        self.state == PdaState::Done
            || (self.channel == SparseChannel::Pir
                && self.block_depth == 0
                && self.state == PdaState::FunctionClosed)
    }

    fn notify_remaining_steps(&mut self, remaining: usize) {
        if self.channel == SparseChannel::Pir
            && remaining <= self.paren_depth + self.block_depth
            && (self.paren_depth + self.block_depth) > 0
        {
            self.force_close_mode = true;
        }
    }
}

pub struct Sampler {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: usize,
    pub repetition_penalty: f32,
    pub constraint: Box<dyn LogitConstraint>,
}

impl Sampler {
    pub fn new(
        temperature: f32,
        top_p: f32,
        top_k: usize,
        repetition_penalty: f32,
        constraint: Box<dyn LogitConstraint>,
    ) -> Self {
        Self {
            temperature,
            top_p,
            top_k,
            repetition_penalty,
            constraint,
        }
    }

    pub fn sample(
        &mut self,
        logits: &mut [f32],
        recent_tokens: &[u32],
        vocab_fn: &dyn Fn(u32) -> String,
    ) -> u32 {
        if self.temperature > 0.0 && self.repetition_penalty != 1.0 {
            let mut seen_recent = std::collections::HashSet::new();
            for &tok in recent_tokens {
                if !seen_recent.insert(tok) || (tok as usize) >= logits.len() {
                    continue;
                }
                if vocab_fn(tok).trim().is_empty() {
                    continue;
                }
                if logits[tok as usize] > 0.0 {
                    logits[tok as usize] /= self.repetition_penalty;
                } else {
                    logits[tok as usize] *= self.repetition_penalty;
                }
            }
        }

        self.constraint.apply_mask(logits, vocab_fn);

        if self.temperature <= 0.0 {
            let mut best_idx = 0;
            let mut best_val = f32::NEG_INFINITY;
            for (i, &l) in logits.iter().enumerate() {
                if l > best_val {
                    best_val = l;
                    best_idx = i;
                }
            }
            return best_idx as u32;
        }

        for l in logits.iter_mut() {
            *l /= self.temperature;
        }

        softmax(logits);

        let mut cumsum = 0.0;
        let mut sorted_logits: Vec<(usize, f32)> = logits.iter().copied().enumerate().collect();
        sorted_logits
            .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if self.top_k > 0 && sorted_logits.len() > self.top_k {
            sorted_logits.truncate(self.top_k);
        }

        let mut filtered = Vec::new();
        for (idx, prob) in sorted_logits {
            if cumsum >= self.top_p {
                break;
            }
            cumsum += prob;
            filtered.push((idx, prob));
        }

        if filtered.is_empty() {
            return 0;
        }

        let sum_prob: f32 = filtered.iter().map(|x| x.1).sum();
        let mut rng_val = (rand_float() * sum_prob).clamp(0.0, sum_prob);

        for &(idx, prob) in &filtered {
            if rng_val <= prob {
                return idx as u32;
            }
            rng_val -= prob;
        }

        filtered[0].0 as u32
    }
}

fn rand_float() -> f32 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(42);
    (nanos % 1000) as f32 / 1000.0
}
