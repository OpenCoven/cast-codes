//! UI-free AI-dialogue data types shared by the server API and auth layers.
//!
//! These types describe the question/answer transcript structure used by the
//! legacy hosted-AI dialogue path. They were extracted out of the Familiar
//! panel's `utils`/`transcript` UI knot so that `server_api`/`auth` no longer
//! depend on the (now-retired) panel view.
use markdown_parser::{parse_markdown, CodeBlockText, FormattedText, FormattedTextLine};
use warpui::elements::{HighlightedHyperlink, MouseStateHandle};

/// A transcript part is a question and answer _pair_. This is to enforce
/// the invariant that every question has an answer.
#[derive(Clone)]
pub struct TranscriptPart {
    pub user: FormattedTranscriptMessage,
    pub assistant: AssistantTranscriptPart,
}

/// The assistant part of a transcript part.
#[derive(Clone)]
pub struct AssistantTranscriptPart {
    pub is_error: bool,
    pub formatted_message: FormattedTranscriptMessage,
    pub copy_all_tooltip_and_button_mouse_handles: Option<(MouseStateHandle, MouseStateHandle)>,
}

/// The information needed to render a single transcript message (whether it be a question or answer).
#[derive(Clone)]
pub struct FormattedTranscriptMessage {
    /// If we can't parse the message as markdown, we can still
    /// use the `raw` field to display it. But we should try to render as markdown.
    pub markdown: Option<Vec<MarkdownSegment>>,
    pub raw: String,
}

impl FormattedTranscriptMessage {
    /// Finds the index of the first code block in the message, if there is one.
    fn first_code_block_index(&self) -> Option<CodeBlockIndex> {
        let segments = self.markdown.as_ref()?;
        segments.iter().find_map(|s| match s {
            MarkdownSegment::CodeBlock { index, .. } => Some(*index),
            _ => None,
        })
    }

    /// Finds the index of the last code block in the message, if there is one.
    fn last_code_block_index(&self) -> Option<CodeBlockIndex> {
        let segments = self.markdown.as_ref()?;
        segments.iter().rev().find_map(|s| match s {
            MarkdownSegment::CodeBlock { index, .. } => Some(*index),
            _ => None,
        })
    }

    /// Finds the index of the next code block after `code_block_index` in the message, if there is one.
    fn next_code_block_index(&self, code_block_index: usize) -> Option<CodeBlockIndex> {
        let segments = self.markdown.as_ref()?;
        segments.iter().find_map(|s| match s {
            MarkdownSegment::CodeBlock { index, .. } => {
                (index.code_block_index == code_block_index + 1).then_some(*index)
            }
            _ => None,
        })
    }

    /// Finds the index of the previous code block before `code_block_index` in the message, if there is one.
    fn prev_code_block_index(&self, code_block_index: usize) -> Option<CodeBlockIndex> {
        if code_block_index == 0 {
            return None;
        }

        let segments = self.markdown.as_ref()?;
        segments.iter().find_map(|s| match s {
            MarkdownSegment::CodeBlock { index, .. } => {
                (index.code_block_index == code_block_index - 1).then_some(*index)
            }
            _ => None,
        })
    }

    /// Returns the raw code block string for the given code block index.
    fn code_for_block(&self, code_block_index: usize) -> Option<&str> {
        let segments = self.markdown.as_ref()?;
        segments.iter().find_map(|s| match s {
            MarkdownSegment::CodeBlock { index, code, .. } => {
                (index.code_block_index == code_block_index).then_some(code.code.as_str())
            }
            _ => None,
        })
    }
}

/// The mouse-state handles needed to render a single code block. Kept as a
/// UI-free leaf type here (a bundle of `warpui` handles) because
/// `MarkdownSegment::CodeBlock` embeds it.
#[derive(Debug, Clone, Default)]
pub struct CodeBlockMouseStateHandles {
    pub play_button: MouseStateHandle,
    pub play_button_tooltip: MouseStateHandle,
    pub copy_button: MouseStateHandle,
    pub copy_button_tooltip: MouseStateHandle,
    pub save_as_workflow_button: MouseStateHandle,
    pub save_as_workflow_button_tooltip: MouseStateHandle,
}

/// A MarkdownSegment differs from a FormattedText in that we intentionally
/// separate out certain markdown elements.
/// For now, only code blocks are rendered differently.
#[derive(Clone)]
pub enum MarkdownSegment {
    CodeBlock {
        index: CodeBlockIndex,
        code: CodeBlockText,
        mouse_state_handles: CodeBlockMouseStateHandles,
    },
    Other {
        /// The formatted text does _not_ contain any of the other
        /// MarkdownSegment's.
        formatted_text: FormattedText,
        highlighted_hyperlink: HighlightedHyperlink,
    },
}

impl TranscriptPart {
    pub fn raw_user_prompt(&self) -> &str {
        self.user.raw.as_str()
    }

    pub fn raw_assistant_answer(&self) -> &str {
        self.assistant.formatted_message.raw.as_str()
    }

    /// Returns the index of the first code block in this transcript part, if there is one.
    pub fn first_code_block_index(&self) -> Option<CodeBlockIndex> {
        self.user
            .first_code_block_index()
            .or_else(|| self.assistant.formatted_message.first_code_block_index())
    }

    /// Returns the index of the last code block in this transcript part, if there is one.
    pub fn last_code_block_index(&self) -> Option<CodeBlockIndex> {
        self.assistant
            .formatted_message
            .last_code_block_index()
            .or_else(|| self.user.last_code_block_index())
    }

    /// Returns the index of the next code block after the given code block index in this transcript part, if there is one.
    pub fn next_code_block_index(
        &self,
        code_block_index: CodeBlockIndex,
    ) -> Option<CodeBlockIndex> {
        match code_block_index.transcript_part_type {
            // Since a transcript part is question -> answer, check if there's a next code block in the question part,
            // otherwise get the first code block in the answer part.
            TranscriptPartSubType::Question => self
                .user
                .next_code_block_index(code_block_index.code_block_index)
                .or_else(|| self.assistant.formatted_message.first_code_block_index()),
            TranscriptPartSubType::Answer => self
                .assistant
                .formatted_message
                .next_code_block_index(code_block_index.code_block_index),
        }
    }

    /// Returns the index of the previous code block before the given code block index in this transcript part, if there is one.
    pub fn prev_code_block_index(
        &self,
        code_block_index: CodeBlockIndex,
    ) -> Option<CodeBlockIndex> {
        match code_block_index.transcript_part_type {
            TranscriptPartSubType::Question => self
                .user
                .prev_code_block_index(code_block_index.code_block_index),
            // Since a transcript part is question -> answer, check if there's a previous code block in the answer part,
            // otherwise get the last code block from the question part.
            TranscriptPartSubType::Answer => self
                .assistant
                .formatted_message
                .prev_code_block_index(code_block_index.code_block_index)
                .or_else(|| self.user.last_code_block_index()),
        }
    }

    pub fn code_for_block(&self, code_block_index: CodeBlockIndex) -> Option<&str> {
        match code_block_index.transcript_part_type {
            TranscriptPartSubType::Question => {
                self.user.code_for_block(code_block_index.code_block_index)
            }
            TranscriptPartSubType::Answer => self
                .assistant
                .formatted_message
                .code_for_block(code_block_index.code_block_index),
        }
    }
}

/// Since a transcript part consists of two sub parts (question and answer),
/// this enum is used to identify which of the two we're referring to.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TranscriptPartSubType {
    Question,
    Answer,
}

impl TranscriptPartSubType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::Answer => "answer",
        }
    }
}

/// A CodeBlockIndex is used to uniquely identify a code block in a transcript.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CodeBlockIndex {
    /// The index into the `trancripts` list.
    transcript_part_index: usize,

    /// Since each transcript part consists of two sub-parts (question & answer),
    /// we need to distinguish which of these sub-parts the code block is in.
    transcript_part_type: TranscriptPartSubType,

    /// A subpart can have > 1 code blocks, so this specifies the exact one.
    code_block_index: usize,
}

impl CodeBlockIndex {
    pub fn new(
        transcript_part_index: usize,
        transcript_part_type: TranscriptPartSubType,
        code_block_index: usize,
    ) -> Self {
        Self {
            transcript_part_index,
            transcript_part_type,
            code_block_index,
        }
    }

    pub fn as_id_str(&self) -> String {
        format!(
            "{}_{}_{}",
            self.transcript_part_index,
            self.transcript_part_type.as_str(),
            self.code_block_index
        )
    }

    pub fn transcript_index(&self) -> usize {
        self.transcript_part_index
    }
}

pub fn markdown_segments_from_text(
    transcript_part_index: usize,
    transcript_part_type: TranscriptPartSubType,
    text: &str,
) -> Option<Vec<MarkdownSegment>> {
    let parsed = parse_markdown(text).ok();
    parsed.map(|p| {
        translate_formatted_text_into_markdown_segments(
            transcript_part_index,
            transcript_part_type,
            p,
        )
    })
}

fn translate_formatted_text_into_markdown_segments(
    transcript_part_index: usize,
    transcript_part_type: TranscriptPartSubType,
    formatted_text: FormattedText,
) -> Vec<MarkdownSegment> {
    // At a high-level, we want to go through the FormattedText and extract
    // all the code-blocks separately from contiguous non-code blocks. We want
    // to do this so that we can render the code-blocks specially. The final
    // result is a set of markdown_segments.
    let mut markdown_segments = vec![];

    // The running non-code block is a contiguous sequence of FormattedTextLine's
    // that _do not_ contain any code blocks.
    let mut running_non_code_block = vec![];

    let mut curr_code_block_index = 0;

    for part in formatted_text.lines {
        match part {
            FormattedTextLine::CodeBlock(mut code) => {
                // If we found a code block, flush out the running non-code-block
                // contiguous sequence into a single markdown segment.
                if !running_non_code_block.is_empty() {
                    markdown_segments.push(MarkdownSegment::Other {
                        formatted_text: FormattedText::new_trimmed(running_non_code_block),
                        highlighted_hyperlink: Default::default(),
                    });
                }

                code.code = code.code.trim().to_string();
                markdown_segments.push(MarkdownSegment::CodeBlock {
                    index: CodeBlockIndex::new(
                        transcript_part_index,
                        transcript_part_type,
                        curr_code_block_index,
                    ),
                    code,
                    mouse_state_handles: Default::default(),
                });
                curr_code_block_index += 1;
                running_non_code_block = vec![];
            }
            _ => {
                // If this is anything other than a code block, tack it onto
                // our running sequence.
                running_non_code_block.push(part);
            }
        }
    }

    // If we had a non-code block sequence that we haven't flushed yet by the end,
    // flush it now.
    if !running_non_code_block.is_empty() {
        markdown_segments.push(MarkdownSegment::Other {
            formatted_text: FormattedText::new_trimmed(running_non_code_block),
            highlighted_hyperlink: Default::default(),
        });
    }

    markdown_segments
}

#[cfg(test)]
#[path = "dialogue_types_tests.rs"]
mod dialogue_types_tests;
