use crate::ast::*;
use crate::error::GenError;
use crate::lexer::{Lexer, LocatedToken, Token};
use std::collections::HashMap;

/// Context for parsing tuplets
struct TupletContext {
    default_duration: Duration,
}

/// Parser for Gen source code
pub struct Parser {
    tokens: Vec<LocatedToken>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<LocatedToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn current(&self) -> Option<&LocatedToken> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> Option<&LocatedToken> {
        let token = self.tokens.get(self.position);
        self.position += 1;
        token
    }

    fn skip_whitespace_and_newlines(&mut self) {
        while let Some(t) = self.current() {
            if t.token == Token::Whitespace || t.token == Token::Newline {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Parse the music content into a Score (metadata already extracted)
    /// mod_points and line_to_measure are passed in from the outer parse function
    pub fn parse_music(&mut self, metadata: Metadata, mod_points: ModPoints, line_to_measure: HashMap<usize, usize>) -> Result<Score, GenError> {
        self.skip_whitespace_and_newlines();

        let mut measures = Vec::new();
        let mut in_slur = false;  // Track whether we're inside a slur across measures
        let mut slur_start_marked = false;  // Track if slur_start has been marked on a note
        let mut pending_tie_stop = false;  // Track whether next note should have tie_stop
        let mut current_ending: Option<Ending> = None;  // Track current ending state across measures

        while self.current().is_some() {
            let (measure_opt, new_slur_state, new_slur_start_marked, new_pending_tie_stop, new_ending) = self.parse_measure(in_slur, slur_start_marked, pending_tie_stop, current_ending)?;
            in_slur = new_slur_state;
            slur_start_marked = new_slur_start_marked;
            pending_tie_stop = new_pending_tie_stop;
            current_ending = new_ending;
            if let Some(measure) = measure_opt {
                measures.push(measure);
            }
            self.skip_whitespace_and_newlines();
        }

        Ok(Score { metadata, measures, mod_points, line_to_measure })
    }

    /// Static method to parse YAML metadata content
    fn parse_yaml_metadata_static(content: &str) -> Result<Metadata, GenError> {
        let parser = Parser::new(vec![]);
        parser.parse_yaml_metadata(content)
    }

    fn parse_yaml_metadata(&self, content: &str) -> Result<Metadata, GenError> {
        let raw: RawMetadata = serde_yaml::from_str(content)
            .map_err(|e| GenError::MetadataError(e.to_string()))?;

        let time_signature = if let Some(ts) = &raw.time_signature {
            self.parse_time_signature(ts)?
        } else {
            TimeSignature::default()
        };

        let key_signature = if let Some(ks) = &raw.key_signature {
            KeySignature::from_str(ks).ok_or_else(|| {
                GenError::MetadataError(format!("Invalid key signature: {}", ks))
            })?
        } else {
            KeySignature::default()
        };

        let written_pitch = if let Some(wp) = &raw.written_pitch {
            self.parse_pitch(wp)?
        } else {
            Pitch::default()
        };

        Ok(Metadata {
            title: raw.title,
            composer: raw.composer,
            time_signature,
            key_signature,
            written_pitch,
        })
    }

    fn parse_time_signature(&self, s: &str) -> Result<TimeSignature, GenError> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(GenError::MetadataError(format!(
                "Invalid time signature: {}",
                s
            )));
        }

        let beats = parts[0]
            .parse()
            .map_err(|_| GenError::MetadataError(format!("Invalid time signature beats: {}", s)))?;
        let beat_type = parts[1]
            .parse()
            .map_err(|_| GenError::MetadataError(format!("Invalid time signature beat type: {}", s)))?;

        Ok(TimeSignature { beats, beat_type })
    }

    fn parse_pitch(&self, s: &str) -> Result<Pitch, GenError> {
        let mut chars = s.chars().peekable();

        // Parse note name
        let note = match chars.next() {
            Some('C') => NoteName::C,
            Some('D') => NoteName::D,
            Some('E') => NoteName::E,
            Some('F') => NoteName::F,
            Some('G') => NoteName::G,
            Some('A') => NoteName::A,
            Some('B') => NoteName::B,
            _ => return Err(GenError::MetadataError(format!("Invalid pitch: {}", s))),
        };

        // Parse octave offset
        let mut octave_offset = 0i8;
        for c in chars {
            match c {
                '^' => octave_offset += 1,
                '_' => octave_offset -= 1,
                _ => {}
            }
        }

        Ok(Pitch { note, octave_offset })
    }

    /// Parse a single measure (one line)
    /// Takes and returns slur state to track slurs across measures, and current ending state
    /// Returns: (Option<Measure>, in_slur, slur_start_marked, pending_tie_stop, current_ending)
    fn parse_measure(&mut self, mut in_slur: bool, mut slur_start_marked: bool, mut next_note_has_tie_stop: bool, current_ending: Option<Ending>) -> Result<(Option<Measure>, bool, bool, bool, Option<Ending>), GenError> {
        let mut elements = Vec::new();
        let mut repeat_start = false;
        let mut repeat_end = false;
        let mut ending: Option<Ending> = current_ending;

        // Check for first/second ending at beginning of measure
        if let Some(t) = self.current() {
            if t.token == Token::FirstEnding {
                ending = Some(Ending::First);
                self.advance();
            } else if t.token == Token::SecondEnding {
                ending = Some(Ending::Second);
                self.advance();
            }
        }

        // Skip whitespace after ending marker
        while let Some(t) = self.current() {
            if t.token == Token::Whitespace {
                self.advance();
            } else {
                break;
            }
        }

        // Check for repeat start at beginning of measure (||:)
        // This can appear on its own line or with notes
        if let Some(t) = self.current() {
            if t.token == Token::RepeatStart {
                repeat_start = true;
                self.advance();
                // Skip whitespace after repeat start
                while let Some(t) = self.current() {
                    if t.token == Token::Whitespace {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        while let Some(t) = self.current() {
            if t.token == Token::Newline {
                self.advance();
                break;
            }

            if t.token == Token::Whitespace {
                self.advance();
                continue;
            }

            // Check for repeat end (:||)
            if t.token == Token::RepeatEnd {
                repeat_end = true;
                self.advance();
                // After repeat end, consume remaining whitespace and newline
                while let Some(t) = self.current() {
                    if t.token == Token::Whitespace {
                        self.advance();
                        continue;
                    }
                    if t.token == Token::Newline {
                        self.advance();
                        break;
                    }
                    // Allow other tokens after :||, don't error
                    break;
                }
                break;
            }

            // Check for slur start
            if t.token == Token::LeftParen {
                self.advance();
                in_slur = true;
                slur_start_marked = false;  // Reset so next note gets slur_start
                continue;
            }

            // Check for slur end
            if t.token == Token::RightParen {
                self.advance();
                // Mark the last note as slur_stop
                if let Some(Element::Note(note)) = elements.last_mut() {
                    note.slur_stop = true;
                }
                in_slur = false;
                slur_start_marked = false;  // Reset for next slur
                continue;
            }

            // Check for bracket groups (rhythm grouping or tuplet)
            // Syntax options:
            //   3[C D E]      - quarter note tuplet (number, then bracket)
            //   /3[C D E]     - eighth note tuplet (rhythm, number, then bracket)
            //   //[C D E F]   - rhythm grouping (rhythm, then bracket, NO number)

            // Save position in case we need to backtrack
            let saved_position = self.position;
            let (t_line, t_col) = (t.line, t.column);

            // Parse optional rhythm modifier
            let (group_duration, group_dotted) = self.parse_rhythm()?;

            // Check for optional tuplet number after rhythm
            let tuplet_number = if let Some(current_t) = self.current() {
                if let Token::Number(n) = current_t.token {
                    self.advance();
                    Some(n)
                } else {
                    None
                }
            } else {
                None
            };

            // Now check if we have a bracket
            if let Some(current_t) = self.current() {
                if current_t.token == Token::LeftBracket {
                    // This is either a tuplet or rhythm grouping
                    let (mut grouped_elements, has_pending_tie) = self.parse_bracket_group(tuplet_number, group_duration, group_dotted)?;

                    // If there's a pending tie_stop, apply it to the first note
                    if next_note_has_tie_stop {
                        if let Some(Element::Note(note)) = grouped_elements.first_mut() {
                            note.tie_stop = true;
                        }
                        next_note_has_tie_stop = false;
                    }

                    // Mark slur_start on first note if we're in a slur and haven't marked it yet
                    if in_slur && !slur_start_marked {
                        if let Some(Element::Note(note)) = grouped_elements.first_mut() {
                            note.slur_start = true;
                            slur_start_marked = true;
                        }
                    }

                    // Check if there's a hyphen after the group (tie from last note)
                    if let Some(t) = self.current() {
                        if t.token == Token::Hyphen {
                            self.advance();
                            if let Some(Element::Note(note)) = grouped_elements.last_mut() {
                                note.tie_start = true;
                            }
                            next_note_has_tie_stop = true;
                        }
                    }

                    // If the bracket group ended with a tie, propagate it
                    if has_pending_tie {
                        next_note_has_tie_stop = true;
                    }

                    elements.extend(grouped_elements);
                    continue;
                }
            }

            // If we got a tuplet number but no bracket, that's an error
            if tuplet_number.is_some() {
                return Err(GenError::ParseError {
                    line: t_line,
                    column: t_col,
                    message: "Expected [ after tuplet number".to_string(),
                });
            }

            // Not a bracket group, restore position and parse as normal element
            self.position = saved_position;

            {
                let mut element = self.parse_element(None)?;

                // Apply tie_stop if pending from previous hyphen
                if next_note_has_tie_stop {
                    if let Element::Note(note) = &mut element {
                        note.tie_stop = true;
                    }
                    next_note_has_tie_stop = false;
                }

                // Mark slur_start on first note if we're in a slur and haven't marked it yet
                if in_slur && !slur_start_marked {
                    if let Element::Note(note) = &mut element {
                        note.slur_start = true;
                        slur_start_marked = true;
                    }
                }

                // Check if there's a hyphen after this note (tie to next note)
                if let Some(t) = self.current() {
                    if t.token == Token::Hyphen {
                        self.advance();
                        if let Element::Note(note) = &mut element {
                            note.tie_start = true;
                        }
                        next_note_has_tie_stop = true;
                    }
                }

                elements.push(element);
            }
        }

        if elements.is_empty() && !repeat_start && !repeat_end && ending.is_none() {
            Ok((None, in_slur, slur_start_marked, next_note_has_tie_stop, ending))
        } else {
            Ok((Some(Measure { elements, repeat_start, repeat_end, ending }), in_slur, slur_start_marked, next_note_has_tie_stop, ending))
        }
    }

    /// Parse a bracket group - either a rhythm grouping (//[C D E F]) or tuplet (3[C D E])
    /// tuplet_number: Some(n) for tuplets, None for rhythm groupings
    /// group_duration: rhythm modifier before the bracket
    /// group_dotted: whether the rhythm has a dot
    /// Returns: (elements, has_pending_tie_stop)
    fn parse_bracket_group(&mut self, tuplet_number: Option<u8>, group_duration: Duration, group_dotted: bool) -> Result<(Vec<Element>, bool), GenError> {
        let (line, column) = self
            .current()
            .map(|t| (t.line, t.column))
            .unwrap_or((0, 0));

        // Consume the opening bracket
        self.advance(); // [

        // Parse the notes inside the bracket, tracking ties and slurs
        let mut raw_elements = Vec::new();
        let mut pending_tie_stop = false;
        let mut in_slur = false;
        let mut slur_start_marked = false;

        while let Some(t) = self.current() {
            if t.token == Token::RightBracket {
                break;
            }
            if t.token == Token::Whitespace {
                self.advance();
                continue;
            }
            if t.token == Token::Newline {
                return Err(GenError::ParseError {
                    line,
                    column,
                    message: "Unexpected newline inside bracket group".to_string(),
                });
            }

            // Check for slur start
            if t.token == Token::LeftParen {
                self.advance();
                in_slur = true;
                slur_start_marked = false;  // Reset so next note gets slur_start
                continue;
            }

            // Check for slur end
            if t.token == Token::RightParen {
                self.advance();
                // Mark the last note as slur_stop
                if let Some(Element::Note(note)) = raw_elements.last_mut() {
                    note.slur_stop = true;
                }
                in_slur = false;
                slur_start_marked = false;  // Reset for next slur
                continue;
            }

            // Parse element without tuplet context for now
            let mut element = self.parse_element(None)?;

            // Apply tie_stop if there was a tie from the previous element
            if pending_tie_stop {
                if let Element::Note(note) = &mut element {
                    note.tie_stop = true;
                }
                pending_tie_stop = false;
            }

            // Mark slur_start on first note if we're in a slur and haven't marked it yet
            if in_slur && !slur_start_marked {
                if let Element::Note(note) = &mut element {
                    note.slur_start = true;
                    slur_start_marked = true;
                }
            }

            // Check for tie (hyphen) after this element
            if let Some(t) = self.current() {
                if t.token == Token::Hyphen {
                    self.advance();
                    if let Element::Note(note) = &mut element {
                        note.tie_start = true;
                    }
                    pending_tie_stop = true;
                }
            }

            raw_elements.push(element);
        }

        // Consume the closing bracket
        if let Some(t) = self.current() {
            if t.token == Token::RightBracket {
                self.advance();
            } else {
                return Err(GenError::ParseError {
                    line,
                    column,
                    message: "Expected closing bracket ]".to_string(),
                });
            }
        }

        if raw_elements.is_empty() {
            return Err(GenError::ParseError {
                line,
                column,
                message: "Bracket group cannot be empty".to_string(),
            });
        }

        // If this is a tuplet (has a number), apply tuplet info to all elements
        if let Some(actual_notes) = tuplet_number {
            let tuplet_context = TupletContext {
                default_duration: group_duration,
            };

            let mut elements = Vec::with_capacity(raw_elements.len());
            let last_idx = raw_elements.len() - 1;

            for (i, element) in raw_elements.into_iter().enumerate() {
                let mut tuplet_info = TupletInfo::new(actual_notes);
                tuplet_info.is_start = i == 0;
                tuplet_info.is_stop = i == last_idx;

                let element_with_tuplet = match element {
                    Element::Note(mut note) => {
                        // If note doesn't have an explicit duration, use the tuplet's default
                        if note.duration == Duration::Quarter {
                            note.duration = tuplet_context.default_duration;
                        }
                        note.tuplet = Some(tuplet_info);
                        Element::Note(note)
                    }
                    Element::Rest { duration, dotted, .. } => {
                        // If rest doesn't have explicit duration, use tuplet's default
                        let final_duration = if duration == Duration::Quarter {
                            tuplet_context.default_duration
                        } else {
                            duration
                        };
                        Element::Rest {
                            duration: final_duration,
                            dotted,
                            tuplet: Some(tuplet_info),
                        }
                    }
                };
                elements.push(element_with_tuplet);
            }

            Ok((elements, pending_tie_stop))
        } else {
            // This is a rhythm grouping (no tuplet number)
            // Apply the group's rhythm to all elements that don't have an explicit rhythm
            let mut elements = Vec::with_capacity(raw_elements.len());

            for element in raw_elements.into_iter() {
                let element_with_rhythm = match element {
                    Element::Note(mut note) => {
                        // If note doesn't have an explicit duration, use the group's rhythm
                        if note.duration == Duration::Quarter && group_duration != Duration::Quarter {
                            note.duration = group_duration;
                            note.dotted = group_dotted;
                        }
                        Element::Note(note)
                    }
                    Element::Rest { duration, dotted, tuplet } => {
                        // If rest doesn't have explicit duration, use group's rhythm
                        let final_duration = if duration == Duration::Quarter && group_duration != Duration::Quarter {
                            group_duration
                        } else {
                            duration
                        };
                        let final_dotted = if duration == Duration::Quarter && group_duration != Duration::Quarter {
                            group_dotted
                        } else {
                            dotted
                        };
                        Element::Rest {
                            duration: final_duration,
                            dotted: final_dotted,
                            tuplet,
                        }
                    }
                };
                elements.push(element_with_rhythm);
            }

            Ok((elements, pending_tie_stop))
        }
    }

    /// Parse a single element (note or rest with rhythm)
    fn parse_element(&mut self, tuplet_info: Option<TupletInfo>) -> Result<Element, GenError> {
        let (line, column) = self
            .current()
            .map(|t| (t.line, t.column))
            .unwrap_or((0, 0));

        // Parse rhythm prefix
        let (duration, dotted) = self.parse_rhythm()?;

        // Parse note or rest
        let current = self.current().ok_or(GenError::ParseError {
            line,
            column,
            message: "Expected note or rest after rhythm".to_string(),
        })?;

        match &current.token {
            Token::Rest => {
                self.advance();
                Ok(Element::Rest { duration, dotted, tuplet: tuplet_info })
            }
            Token::NoteA | Token::NoteB | Token::NoteC | Token::NoteD | Token::NoteE
            | Token::NoteF | Token::NoteG => {
                let name = self.parse_note_name()?;
                let (accidental, octave) = self.parse_pitch_modifiers();

                Ok(Element::Note(Note {
                    name,
                    accidental,
                    octave,
                    duration,
                    dotted,
                    tuplet: tuplet_info,
                    tie_start: false,
                    tie_stop: false,
                    slur_start: false,
                    slur_stop: false,
                }))
            }
            _ => Err(GenError::ParseError {
                line: current.line,
                column: current.column,
                message: format!("Expected note or rest, found {:?}", current.token),
            }),
        }
    }

    /// Parse rhythm modifiers and return (Duration, dotted)
    fn parse_rhythm(&mut self) -> Result<(Duration, bool), GenError> {
        let mut slash_count = 0;
        let mut has_pipe = false;
        let mut has_o = false;
        let mut dotted = false;

        // Count rhythm modifiers
        loop {
            let Some(t) = self.current() else { break };

            match &t.token {
                Token::Slash => {
                    self.advance();
                    slash_count += 1;
                }
                Token::Pipe => {
                    self.advance();
                    has_pipe = true;
                }
                Token::SmallO => {
                    self.advance();
                    has_o = true;
                }
                Token::Asterisk => {
                    self.advance();
                    dotted = true;
                }
                _ => break,
            }
        }

        // Determine duration based on modifiers
        let duration = match (slash_count, has_pipe, has_o) {
            (0, false, true) => Duration::Whole,        // o
            (0, true, true) => Duration::Half,          // |o
            (0, false, false) | (0, true, false) => Duration::Quarter, // (none) or |
            (1, false, false) => Duration::Eighth,      // /
            (2, false, false) => Duration::Sixteenth,   // //
            (3, false, false) => Duration::ThirtySecond, // ///
            _ => Duration::Quarter, // fallback
        };

        Ok((duration, dotted))
    }

    fn parse_note_name(&mut self) -> Result<NoteName, GenError> {
        let current = self.current().ok_or(GenError::ParseError {
            line: 0,
            column: 0,
            message: "Expected note name".to_string(),
        })?;

        let name = match &current.token {
            Token::NoteA => NoteName::A,
            Token::NoteB => NoteName::B,
            Token::NoteC => NoteName::C,
            Token::NoteD => NoteName::D,
            Token::NoteE => NoteName::E,
            Token::NoteF => NoteName::F,
            Token::NoteG => NoteName::G,
            _ => {
                return Err(GenError::ParseError {
                    line: current.line,
                    column: current.column,
                    message: format!("Expected note name, found {:?}", current.token),
                })
            }
        };

        self.advance();
        Ok(name)
    }

    fn parse_pitch_modifiers(&mut self) -> (Accidental, Octave) {
        let mut accidental = Accidental::Natural;

        // Parse accidental (#, b, or %)
        if let Some(t) = self.current() {
            match &t.token {
                Token::Sharp => {
                    accidental = Accidental::Sharp;
                    self.advance();
                }
                Token::Flat => {
                    accidental = Accidental::Flat;
                    self.advance();
                }
                Token::Natural => {
                    accidental = Accidental::ForceNatural;
                    self.advance();
                }
                _ => {}
            }
        }

        // Parse octave modifiers (_ or ^)
        let mut octave_offset = 0i8;
        while let Some(t) = self.current() {
            match &t.token {
                Token::Underscore => {
                    octave_offset -= 1;
                    self.advance();
                }
                Token::Caret => {
                    octave_offset += 1;
                    self.advance();
                }
                _ => break,
            }
        }

        let octave = match octave_offset {
            i if i <= -2 => Octave::DoubleLow,
            -1 => Octave::Low,
            0 => Octave::Middle,
            1 => Octave::High,
            _ => Octave::DoubleHigh,
        };

        (accidental, octave)
    }
}

/// Extract metadata block from source (can be at top or bottom)
/// Returns (metadata_content, remaining_source)
fn extract_metadata(source: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = source.lines().collect();

    // Find the metadata block (between --- markers)
    let mut start_idx = None;
    let mut end_idx = None;

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "---" {
            if start_idx.is_none() {
                start_idx = Some(i);
            } else {
                end_idx = Some(i);
                break;
            }
        }
    }

    match (start_idx, end_idx) {
        (Some(start), Some(end)) => {
            // Extract metadata content (between the --- markers)
            let metadata_content: String = lines[start + 1..end].join("\n");

            // Remove metadata lines from the source
            let remaining: Vec<&str> = lines[..start]
                .iter()
                .chain(lines[end + 1..].iter())
                .copied()
                .collect();

            (Some(metadata_content), remaining.join("\n"))
        }
        _ => (None, source.to_string()),
    }
}

/// Extract mod points from inline annotations in the source.
/// Annotations are in the format: @Eb:^ or @Bb:_
/// Returns (ModPoints, line_to_measure mapping)
/// The line_to_measure maps 1-indexed source line numbers to measure indices.
fn extract_mod_points(source: &str) -> (ModPoints, HashMap<usize, usize>) {
    let mut mod_points = ModPoints::default();
    let mut line_to_measure: HashMap<usize, usize> = HashMap::new();
    let mut measure_index = 0;
    let mut in_metadata = false;

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1; // 1-indexed
        let trimmed = line.trim();

        // Track metadata blocks (between --- markers)
        if trimmed == "---" {
            in_metadata = !in_metadata;
            continue;
        }

        // Skip lines inside metadata blocks
        if in_metadata {
            continue;
        }

        // Check if this line has any music content (not just whitespace/annotations)
        // Find the first @ that starts a mod point annotation
        let content_before_annotation = if let Some(at_pos) = line.find('@') {
            // Check if this @ is followed by Eb: or Bb: pattern
            let rest = &line[at_pos..];
            if rest.len() >= 4 && (rest[1..].to_lowercase().starts_with("eb:") || rest[1..].to_lowercase().starts_with("bb:")) {
                &line[..at_pos]
            } else {
                line
            }
        } else {
            line
        };

        // Skip lines that are only whitespace
        let content_trimmed = content_before_annotation.trim();
        if !content_trimmed.is_empty() {
            line_to_measure.insert(line_num, measure_index);
            measure_index += 1;
        }

        // Parse mod points from annotations
        // Look for patterns like @Eb:^ or @Bb:_
        for (i, _) in line.match_indices('@') {
            let rest = &line[i + 1..]; // Skip the @

            // Parse Group:modifier pattern
            if let Some((group_str, remainder)) = rest.split_once(':') {
                let group_str = group_str.trim();
                if let Some(group) = InstrumentGroup::from_str(group_str) {
                    // Get the modifier (first non-whitespace chars after :)
                    let modifier = remainder.split_whitespace().next().unwrap_or("");
                    let shift = match modifier {
                        "^" => Some(1i8),
                        "_" => Some(-1i8),
                        "^^" => Some(2i8),
                        "__" => Some(-2i8),
                        _ => None,
                    };
                    if let Some(shift) = shift {
                        mod_points.set_shift(line_num, group, shift);
                    }
                }
            }
        }
    }

    (mod_points, line_to_measure)
}

/// Main parsing function
pub fn parse(source: &str) -> Result<Score, GenError> {
    // Extract mod points from comments first (before any other processing)
    // This needs the original source to get correct line numbers
    let (mod_points, line_to_measure) = extract_mod_points(source);

    // Extract metadata block (can be anywhere in the file)
    let (metadata_content, music_source) = extract_metadata(source);

    // Parse metadata
    let metadata = if let Some(content) = metadata_content {
        Parser::parse_yaml_metadata_static(&content)?
    } else {
        Metadata::default()
    };

    // Parse music content
    let mut lexer = Lexer::new(&music_source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_music(metadata, mod_points, line_to_measure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_measure() {
        let score = parse("C D E F").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_with_metadata() {
        let source = r#"---
title: Test Song
time-signature: 3/4
---
C D E"#;
        let score = parse(source).unwrap();
        assert_eq!(score.metadata.title, Some("Test Song".to_string()));
        assert_eq!(score.metadata.time_signature.beats, 3);
        assert_eq!(score.metadata.time_signature.beat_type, 4);
    }

    #[test]
    fn test_with_metadata_at_bottom() {
        let source = r#"C D E
G A B
---
title: Bottom Metadata
time-signature: 6/8
---"#;
        let score = parse(source).unwrap();
        assert_eq!(score.metadata.title, Some("Bottom Metadata".to_string()));
        assert_eq!(score.metadata.time_signature.beats, 6);
        assert_eq!(score.metadata.time_signature.beat_type, 8);
        assert_eq!(score.measures.len(), 2);
    }

    #[test]
    fn test_rhythm_modifiers() {
        let score = parse("/C |oD oE").unwrap();
        let elements = &score.measures[0].elements;

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.duration, Duration::Eighth);
        }
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.duration, Duration::Half);
        }
        if let Element::Note(n) = &elements[2] {
            assert_eq!(n.duration, Duration::Whole);
        }
    }

    #[test]
    fn test_dotted_quarter_rest_asterisk_dollar() {
        // *$ - dotted quarter rest (asterisk before dollar)
        let score = parse("*$").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 1);

        if let Element::Rest { duration, dotted, .. } = &elements[0] {
            assert_eq!(*duration, Duration::Quarter);
            assert!(*dotted, "Rest should be dotted");
        } else {
            panic!("Expected rest");
        }
    }

    #[test]
    fn test_dotted_quarter_rest_pipe_asterisk_dollar() {
        // |*$ - dotted quarter rest (pipe + asterisk before dollar)
        let score = parse("|*$").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 1);

        if let Element::Rest { duration, dotted, .. } = &elements[0] {
            assert_eq!(*duration, Duration::Quarter);
            assert!(*dotted, "Rest should be dotted");
        } else {
            panic!("Expected rest");
        }
    }

    #[test]
    fn test_dotted_half_rest() {
        // |o*$ - dotted half rest
        let score = parse("|o*$").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 1);

        if let Element::Rest { duration, dotted, .. } = &elements[0] {
            assert_eq!(*duration, Duration::Half);
            assert!(*dotted, "Rest should be dotted");
        } else {
            panic!("Expected rest");
        }
    }

    #[test]
    fn test_dotted_eighth_rest() {
        // /*$ - dotted eighth rest
        let score = parse("/*$").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 1);

        if let Element::Rest { duration, dotted, .. } = &elements[0] {
            assert_eq!(*duration, Duration::Eighth);
            assert!(*dotted, "Rest should be dotted");
        } else {
            panic!("Expected rest");
        }
    }

    #[test]
    fn test_triplet_parsing() {
        // Quarter note triplet: 3[C D E]
        let score = parse("3[C D E]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        // Check that all notes have triplet info
        for (i, element) in elements.iter().enumerate() {
            if let Element::Note(n) = element {
                assert!(n.tuplet.is_some(), "Note {} should have tuplet info", i);
                let tuplet = n.tuplet.unwrap();
                assert_eq!(tuplet.actual_notes, 3);
                assert_eq!(tuplet.normal_notes, 2);

                // Check start/stop markers
                if i == 0 {
                    assert!(tuplet.is_start);
                    assert!(!tuplet.is_stop);
                } else if i == 2 {
                    assert!(!tuplet.is_start);
                    assert!(tuplet.is_stop);
                } else {
                    assert!(!tuplet.is_start);
                    assert!(!tuplet.is_stop);
                }
            }
        }
    }

    #[test]
    fn test_eighth_note_triplet() {
        // Eighth note triplet: /3[C D E]
        let score = parse("/3[C D E]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        for element in elements.iter() {
            if let Element::Note(n) = element {
                assert_eq!(n.duration, Duration::Eighth);
                assert!(n.tuplet.is_some());
            }
        }
    }

    #[test]
    fn test_triplet_with_mixed_notes() {
        // Triplet with explicit rhythm on first note
        let score = parse("3[/C D E]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        // First note should be eighth (explicit), others should be quarter (default)
        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.duration, Duration::Eighth);
        }
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.duration, Duration::Quarter);
        }
    }

    #[test]
    fn test_rhythm_grouping_sixteenth() {
        // Sixteenth note grouping: //[C D E F]
        let score = parse("//[C D E F]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        // All notes should be sixteenth notes
        for element in elements.iter() {
            if let Element::Note(n) = element {
                assert_eq!(n.duration, Duration::Sixteenth);
                assert!(n.tuplet.is_none(), "Rhythm grouping should not have tuplet info");
            }
        }
    }

    #[test]
    fn test_rhythm_grouping_eighth() {
        // Eighth note grouping: /[C D E F]
        let score = parse("/[C D E F]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        for element in elements.iter() {
            if let Element::Note(n) = element {
                assert_eq!(n.duration, Duration::Eighth);
                assert!(n.tuplet.is_none());
            }
        }
    }

    #[test]
    fn test_rhythm_grouping_with_override() {
        // Rhythm grouping with one note overriding: //[C D /E F]
        let score = parse("//[C D /E F]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        // First, second, and fourth should be sixteenth
        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.duration, Duration::Sixteenth);
        }
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.duration, Duration::Sixteenth);
        }
        // Third should be eighth (explicit override)
        if let Element::Note(n) = &elements[2] {
            assert_eq!(n.duration, Duration::Eighth);
        }
        if let Element::Note(n) = &elements[3] {
            assert_eq!(n.duration, Duration::Sixteenth);
        }
    }

    #[test]
    fn test_quintuplet() {
        // Quintuplet: 5[C D E F G]
        let score = parse("5[C D E F G]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 5);

        for (i, element) in elements.iter().enumerate() {
            if let Element::Note(n) = element {
                assert!(n.tuplet.is_some());
                let tuplet = n.tuplet.unwrap();
                assert_eq!(tuplet.actual_notes, 5);
                assert_eq!(tuplet.normal_notes, 4);

                if i == 0 {
                    assert!(tuplet.is_start);
                } else if i == 4 {
                    assert!(tuplet.is_stop);
                }
            }
        }
    }

    #[test]
    fn test_sextuplet() {
        // Sextuplet: 6[C D E F G A]
        let score = parse("6[C D E F G A]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 6);

        for element in elements.iter() {
            if let Element::Note(n) = element {
                assert!(n.tuplet.is_some());
                let tuplet = n.tuplet.unwrap();
                assert_eq!(tuplet.actual_notes, 6);
                assert_eq!(tuplet.normal_notes, 4);
            }
        }
    }

    #[test]
    fn test_simple_tie() {
        // C tied to D
        let score = parse("C-D").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 2);

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::C);
            assert!(n.tie_start, "First note should have tie_start");
            assert!(!n.tie_stop, "First note should not have tie_stop");
        } else {
            panic!("Expected note");
        }

        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.name, NoteName::D);
            assert!(!n.tie_start, "Second note should not have tie_start");
            assert!(n.tie_stop, "Second note should have tie_stop");
        } else {
            panic!("Expected note");
        }
    }

    #[test]
    fn test_chained_ties() {
        // C tied to D tied to E
        let score = parse("C-D-E").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        if let Element::Note(n) = &elements[0] {
            assert!(n.tie_start);
            assert!(!n.tie_stop);
        }

        if let Element::Note(n) = &elements[1] {
            assert!(n.tie_start, "Middle note should have tie_start");
            assert!(n.tie_stop, "Middle note should have tie_stop");
        }

        if let Element::Note(n) = &elements[2] {
            assert!(!n.tie_start);
            assert!(n.tie_stop);
        }
    }

    #[test]
    fn test_tie_with_rhythm_modifiers() {
        // Eighth note C tied to quarter note D
        let score = parse("/C-D").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 2);

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.duration, Duration::Eighth);
            assert!(n.tie_start);
        }

        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.duration, Duration::Quarter);
            assert!(n.tie_stop);
        }
    }

    #[test]
    fn test_tie_mixed_with_regular_notes() {
        // Tie followed by regular notes
        let score = parse("C-D E F").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        if let Element::Note(n) = &elements[0] {
            assert!(n.tie_start);
        }

        if let Element::Note(n) = &elements[1] {
            assert!(n.tie_stop);
        }

        // E and F should have no ties
        if let Element::Note(n) = &elements[2] {
            assert!(!n.tie_start);
            assert!(!n.tie_stop);
        }

        if let Element::Note(n) = &elements[3] {
            assert!(!n.tie_start);
            assert!(!n.tie_stop);
        }
    }

    #[test]
    fn test_tie_across_measures() {
        // Tie that spans two measures: last note of measure 1 tied to first note of measure 2
        let score = parse("C D E F-\nG A B C^").unwrap();

        assert_eq!(score.measures.len(), 2);
        assert_eq!(score.measures[0].elements.len(), 4);
        assert_eq!(score.measures[1].elements.len(), 4);

        // Last note of first measure (F) should have tie_start
        if let Element::Note(n) = &score.measures[0].elements[3] {
            assert_eq!(n.name, NoteName::F);
            assert!(n.tie_start, "Last note of first measure should have tie_start");
            assert!(!n.tie_stop);
        } else {
            panic!("Expected note");
        }

        // First note of second measure (G) should have tie_stop
        if let Element::Note(n) = &score.measures[1].elements[0] {
            assert_eq!(n.name, NoteName::G);
            assert!(!n.tie_start, "First note of second measure should not have tie_start");
            assert!(n.tie_stop, "First note of second measure should have tie_stop");
        } else {
            panic!("Expected note");
        }

        // Other notes should have no ties
        if let Element::Note(n) = &score.measures[1].elements[1] {
            assert!(!n.tie_start);
            assert!(!n.tie_stop);
        }
    }

    #[test]
    fn test_repeat_start() {
        let score = parse("||: C D E F").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert!(score.measures[0].repeat_start);
        assert!(!score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_repeat_end() {
        let score = parse("C D E F :||").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert!(!score.measures[0].repeat_start);
        assert!(score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_repeat_both_same_measure() {
        let score = parse("||: C D E F :||").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert!(score.measures[0].repeat_start);
        assert!(score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_repeat_across_measures() {
        let score = parse("||: C D E F\nG A B C^ :||").unwrap();
        assert_eq!(score.measures.len(), 2);
        assert!(score.measures[0].repeat_start);
        assert!(!score.measures[0].repeat_end);
        assert!(!score.measures[1].repeat_start);
        assert!(score.measures[1].repeat_end);
    }

    #[test]
    fn test_simple_slur() {
        // (C D E) - three slurred notes
        let score = parse("(C D E)").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        // First note should have slur_start
        if let Element::Note(n) = &elements[0] {
            assert!(n.slur_start, "First note should have slur_start");
            assert!(!n.slur_stop, "First note should not have slur_stop");
        } else {
            panic!("Expected note");
        }

        // Middle note should have neither
        if let Element::Note(n) = &elements[1] {
            assert!(!n.slur_start, "Middle note should not have slur_start");
            assert!(!n.slur_stop, "Middle note should not have slur_stop");
        } else {
            panic!("Expected note");
        }

        // Last note should have slur_stop
        if let Element::Note(n) = &elements[2] {
            assert!(!n.slur_start, "Last note should not have slur_start");
            assert!(n.slur_stop, "Last note should have slur_stop");
        } else {
            panic!("Expected note");
        }
    }

    #[test]
    fn test_slur_with_accidentals_and_octaves() {
        // (Bb_ D F) - slur with flat and octave modifier
        let score = parse("(Bb_ D F)").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::B);
            assert_eq!(n.accidental, Accidental::Flat);
            assert_eq!(n.octave, Octave::Low);
            assert!(n.slur_start);
        }

        if let Element::Note(n) = &elements[2] {
            assert_eq!(n.name, NoteName::F);
            assert!(n.slur_stop);
        }
    }

    #[test]
    fn test_slur_followed_by_regular_note() {
        // (C D E) F - slur followed by regular note
        let score = parse("(C D E) F").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        // First three notes should be slurred
        if let Element::Note(n) = &elements[0] {
            assert!(n.slur_start);
        }
        if let Element::Note(n) = &elements[2] {
            assert!(n.slur_stop);
        }

        // Fourth note should have no slur
        if let Element::Note(n) = &elements[3] {
            assert!(!n.slur_start);
            assert!(!n.slur_stop);
        }
    }

    #[test]
    fn test_slur_two_notes() {
        // (C D) - two note slur (first note has both start and stop for rendering)
        let score = parse("(C D)").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 2);

        if let Element::Note(n) = &elements[0] {
            assert!(n.slur_start);
            assert!(!n.slur_stop);
        }

        if let Element::Note(n) = &elements[1] {
            assert!(!n.slur_start);
            assert!(n.slur_stop);
        }
    }

    #[test]
    fn test_slur_with_rhythm_modifiers() {
        // (/C /D /E) - eighth note slur
        let score = parse("(/C /D /E)").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        for element in elements {
            if let Element::Note(n) = element {
                assert_eq!(n.duration, Duration::Eighth);
            }
        }

        if let Element::Note(n) = &elements[0] {
            assert!(n.slur_start);
        }
        if let Element::Note(n) = &elements[2] {
            assert!(n.slur_stop);
        }
    }

    #[test]
    fn test_slur_across_measures() {
        // Slur that spans two measures
        let score = parse("(C D E F\nG A B C^)").unwrap();

        assert_eq!(score.measures.len(), 2);
        assert_eq!(score.measures[0].elements.len(), 4);
        assert_eq!(score.measures[1].elements.len(), 4);

        // First note of first measure should have slur_start
        if let Element::Note(n) = &score.measures[0].elements[0] {
            assert!(n.slur_start, "First note should have slur_start");
            assert!(!n.slur_stop);
        }

        // Last note of first measure should NOT have slur_stop (slur continues)
        if let Element::Note(n) = &score.measures[0].elements[3] {
            assert!(!n.slur_start);
            assert!(!n.slur_stop, "Last note of first measure should not have slur_stop");
        }

        // First note of second measure should NOT have slur_start (slur continues)
        if let Element::Note(n) = &score.measures[1].elements[0] {
            assert!(!n.slur_start, "First note of second measure should not have slur_start");
            assert!(!n.slur_stop);
        }

        // Last note of second measure should have slur_stop
        if let Element::Note(n) = &score.measures[1].elements[3] {
            assert!(!n.slur_start);
            assert!(n.slur_stop, "Last note should have slur_stop");
        }
    }

    #[test]
    fn test_first_ending_parsing() {
        // 1. measure with repeat end
        let score = parse("1. C C C C :||").unwrap();

        assert_eq!(score.measures.len(), 1);
        assert_eq!(score.measures[0].ending, Some(Ending::First));
        assert!(score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_second_ending_parsing() {
        // 2. measure without repeat
        let score = parse("2. C C C C").unwrap();

        assert_eq!(score.measures.len(), 1);
        assert_eq!(score.measures[0].ending, Some(Ending::Second));
        assert!(!score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_first_and_second_endings() {
        // Full volta bracket pattern
        let source = "oF\n1. C C C C :||\n2. D D D D";
        let score = parse(source).unwrap();

        assert_eq!(score.measures.len(), 3);

        // First measure - no ending
        assert_eq!(score.measures[0].ending, None);

        // Second measure - first ending with repeat
        assert_eq!(score.measures[1].ending, Some(Ending::First));
        assert!(score.measures[1].repeat_end);

        // Third measure - second ending without repeat
        assert_eq!(score.measures[2].ending, Some(Ending::Second));
        assert!(!score.measures[2].repeat_end);
    }

    #[test]
    fn test_force_natural() {
        // C% - C with explicit natural sign
        let score = parse("C% D E F").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::C);
            assert_eq!(n.accidental, Accidental::ForceNatural);
        } else {
            panic!("Expected note");
        }

        // Other notes should have no accidental
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.accidental, Accidental::Natural);
        }
    }

    #[test]
    fn test_force_natural_with_octave() {
        // F%^ - F natural, octave up
        let score = parse("F%^").unwrap();
        let elements = &score.measures[0].elements;

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::F);
            assert_eq!(n.accidental, Accidental::ForceNatural);
            assert_eq!(n.octave, Octave::High);
        } else {
            panic!("Expected note");
        }
    }

    #[test]
    fn test_mod_points_single() {
        // Single mod point on line 1
        let score = parse("C D E F @Eb:^").unwrap();

        assert_eq!(score.measures.len(), 1);
        assert_eq!(score.measures[0].elements.len(), 4);

        // Check mod point was extracted
        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Eb), Some(1));
        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Bb), None);
    }

    #[test]
    fn test_mod_points_multiple_groups() {
        // Multiple mod points on same line
        let score = parse("C D E F @Eb:^ @Bb:_").unwrap();

        assert_eq!(score.measures.len(), 1);

        // Check both mod points
        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Eb), Some(1));
        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Bb), Some(-1));
    }

    #[test]
    fn test_mod_points_multiple_lines() {
        // Mod points on different lines
        let score = parse("C D E F @Eb:^\nG A B C @Bb:_").unwrap();

        assert_eq!(score.measures.len(), 2);

        // Line 1 has Eb up
        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Eb), Some(1));
        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Bb), None);

        // Line 2 has Bb down
        assert_eq!(score.mod_points.get_shift(2, InstrumentGroup::Eb), None);
        assert_eq!(score.mod_points.get_shift(2, InstrumentGroup::Bb), Some(-1));
    }

    #[test]
    fn test_mod_points_down_octave() {
        let score = parse("C D E F @Eb:_").unwrap();

        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Eb), Some(-1));
    }

    #[test]
    fn test_line_to_measure_mapping() {
        let score = parse("C D E F\nG A B C").unwrap();

        // Line 1 maps to measure 0
        assert_eq!(score.line_to_measure.get(&1), Some(&0));
        // Line 2 maps to measure 1
        assert_eq!(score.line_to_measure.get(&2), Some(&1));
    }

    #[test]
    fn test_line_to_measure_with_metadata() {
        let source = "---\ntitle: Test\n---\nC D E F\nG A B C";
        let score = parse(source).unwrap();

        assert_eq!(score.measures.len(), 2);
        // Lines 1-3 are metadata, so music starts at line 4
        assert_eq!(score.line_to_measure.get(&4), Some(&0));
        assert_eq!(score.line_to_measure.get(&5), Some(&1));
    }

    #[test]
    fn test_mod_points_with_metadata_at_bottom() {
        // Like spain.gen - metadata at bottom, mod points on music lines
        let source = "C D E F @Eb:^\nG A B C\n---\ntitle: Test\n---";
        let score = parse(source).unwrap();

        assert_eq!(score.measures.len(), 2);
        // Line 1 has music and mod point
        assert_eq!(score.line_to_measure.get(&1), Some(&0));
        assert_eq!(score.mod_points.get_shift(1, InstrumentGroup::Eb), Some(1));
        // Line 2 has music, no mod point
        assert_eq!(score.line_to_measure.get(&2), Some(&1));
        assert_eq!(score.mod_points.get_shift(2, InstrumentGroup::Eb), None);
    }

    #[test]
    fn test_slur_in_rhythm_grouping() {
        // Slur inside a rhythm grouping: //[(G_ F#_ G_) G]
        let score = parse("//[(G_ F#_ G_) G]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        // All notes should be sixteenth notes
        for element in elements.iter() {
            if let Element::Note(n) = element {
                assert_eq!(n.duration, Duration::Sixteenth);
            }
        }

        // First note should have slur_start
        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::G);
            assert_eq!(n.octave, Octave::Low);
            assert!(n.slur_start, "First note should have slur_start");
            assert!(!n.slur_stop);
        }

        // Second note should have neither
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.name, NoteName::F);
            assert_eq!(n.accidental, Accidental::Sharp);
            assert_eq!(n.octave, Octave::Low);
            assert!(!n.slur_start);
            assert!(!n.slur_stop);
        }

        // Third note should have slur_stop
        if let Element::Note(n) = &elements[2] {
            assert_eq!(n.name, NoteName::G);
            assert_eq!(n.octave, Octave::Low);
            assert!(!n.slur_start);
            assert!(n.slur_stop, "Third note should have slur_stop");
        }

        // Fourth note should have no slur
        if let Element::Note(n) = &elements[3] {
            assert_eq!(n.name, NoteName::G);
            assert_eq!(n.octave, Octave::Middle);
            assert!(!n.slur_start);
            assert!(!n.slur_stop);
        }
    }

    #[test]
    fn test_slur_in_tuplet() {
        // Slur inside a tuplet: /3[(C D) E]
        let score = parse("/3[(C D) E]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        // All notes should have tuplet info
        for element in elements.iter() {
            if let Element::Note(n) = element {
                assert!(n.tuplet.is_some());
                assert_eq!(n.duration, Duration::Eighth);
            }
        }

        // First note should have slur_start
        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::C);
            assert!(n.slur_start);
            assert!(!n.slur_stop);
        }

        // Second note should have slur_stop
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.name, NoteName::D);
            assert!(!n.slur_start);
            assert!(n.slur_stop);
        }

        // Third note should have no slur
        if let Element::Note(n) = &elements[2] {
            assert_eq!(n.name, NoteName::E);
            assert!(!n.slur_start);
            assert!(!n.slur_stop);
        }
    }

    #[test]
    fn test_multiple_slurs_in_rhythm_grouping() {
        // Multiple slurs in one rhythm grouping: //[(C D) (E F)]
        let score = parse("//[(C D) (E F)]").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        // First slur: C and D
        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::C);
            assert!(n.slur_start);
            assert!(!n.slur_stop);
        }
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.name, NoteName::D);
            assert!(!n.slur_start);
            assert!(n.slur_stop);
        }

        // Second slur: E and F
        if let Element::Note(n) = &elements[2] {
            assert_eq!(n.name, NoteName::E);
            assert!(n.slur_start);
            assert!(!n.slur_stop);
        }
        if let Element::Note(n) = &elements[3] {
            assert_eq!(n.name, NoteName::F);
            assert!(!n.slur_start);
            assert!(n.slur_stop);
        }
    }
}
