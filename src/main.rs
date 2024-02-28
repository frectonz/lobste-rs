use std::io;

use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use reqwest::blocking::Client;
use serde::Deserialize;
use tui::{
    backend::{Backend, CrosstermBackend},
    style::{Modifier, Style},
    text,
    widgets::{Block, List, ListItem, Paragraph},
    Frame, Terminal,
};

#[allow(dead_code)]

type Stories = Vec<Story>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Story {
    #[serde(rename = "short_id")]
    pub short_id: String,
    #[serde(rename = "short_id_url")]
    pub short_id_url: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub title: String,
    pub url: String,
    pub score: i64,
    pub flags: i64,
    #[serde(rename = "comment_count")]
    pub comment_count: i64,
    pub description: String,
    #[serde(rename = "description_plain")]
    pub description_plain: String,
    #[serde(rename = "comments_url")]
    pub comments_url: String,
    #[serde(rename = "submitter_user")]
    pub submitter_user: SubmitterUser,
    pub tags: Vec<String>,
}

impl Story {
    fn url(&self) -> &str {
        if self.url.is_empty() {
            &self.short_id_url
        } else {
            &self.url
        }
    }

    fn url_span(&self) -> text::Span {
        text::Span::styled(
            self.url(),
            Style::default()
                .fg(tui::style::Color::Blue)
                .add_modifier(tui::style::Modifier::UNDERLINED),
        )
    }

    fn title_span(&self, selected: bool) -> text::Span {
        text::Span::styled(
            &self.title,
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(if selected {
                    tui::style::Color::Green
                } else {
                    tui::style::Color::White
                }),
        )
    }

    fn score_span(&self) -> text::Span {
        text::Span::styled(
            format!("⧋ {: <4}", self.score),
            Style::default().fg(tui::style::Color::Yellow),
        )
    }
}

struct StoryWidget<'a> {
    story: &'a Story,
    selected: bool,
}

impl<'a> StoryWidget<'a> {
    fn new((story, selected): (&'a Story, bool)) -> Self {
        Self { story, selected }
    }
}

impl<'a> Into<ListItem<'a>> for StoryWidget<'a> {
    fn into(self) -> ListItem<'a> {
        let selected_indicator = if self.selected { "► " } else { "  " };

        let span = text::Spans::from(vec![
            selected_indicator.into(),
            self.story.score_span(),
            " ".into(),
            self.story.title_span(self.selected),
            " ".into(),
            self.story.url_span(),
        ]);

        ListItem::new(span)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitterUser {
    pub username: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "is_admin")]
    pub is_admin: bool,
    pub about: String,
    #[serde(rename = "is_moderator")]
    pub is_moderator: bool,
    pub karma: Option<i64>,
    #[serde(rename = "avatar_url")]
    pub avatar_url: String,
    #[serde(rename = "invited_by_user")]
    pub invited_by_user: Option<String>,
    #[serde(rename = "github_username")]
    pub github_username: Option<String>,
    #[serde(rename = "twitter_username")]
    pub twitter_username: Option<String>,
    #[serde(rename = "keybase_signatures")]
    #[serde(default)]
    pub keybase_signatures: Vec<KeybaseSignature>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeybaseSignature {
    #[serde(rename = "kb_username")]
    pub kb_username: String,
    #[serde(rename = "sig_hash")]
    pub sig_hash: String,
}

const BANNER: &str = r#"
 ████           █████              █████                                 
░░███          ░░███              ░░███                                  
 ░███   ██████  ░███████   █████  ███████    ██████     ████████   █████ 
 ░███  ███░░███ ░███░░███ ███░░  ░░░███░    ███░░███   ░░███░░███ ███░░  
 ░███ ░███ ░███ ░███ ░███░░█████   ░███    ░███████     ░███ ░░░ ░░█████ 
 ░███ ░███ ░███ ░███ ░███ ░░░░███  ░███ ███░███░░░      ░███      ░░░░███
 █████░░██████  ████████  ██████   ░░█████ ░░██████  ██ █████     ██████ 
░░░░░  ░░░░░░  ░░░░░░░░  ░░░░░░     ░░░░░   ░░░░░░  ░░ ░░░░░     ░░░░░░  
                                                                         
                                                                         
"#;

struct App {
    client: Client,
    stories: Stories,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    selected_story_index: usize,
    page: usize,
}

impl App {
    fn init() -> Result<Self> {
        let client = Client::builder().build()?;
        let mut terminal = App::init_screen()?;

        let stories: Stories = client
            .get("https://lobste.rs/newest.json")
            .send()?
            .json()
            .map_err(|e| {
                reset_terminal(&mut terminal);
                e
            })?;

        Ok(Self {
            client,
            stories,
            terminal,
            selected_story_index: 0,
            page: 1,
        })
    }

    fn init_screen() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Terminal::new(CrosstermBackend::new(io::stdout()))?)
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.terminal.draw(|f| {
                Self::draw_stories(f, &self.stories, self.selected_story_index, self.page)
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Down => {
                        self.selected_story_index =
                            if self.selected_story_index == self.stories.len() - 1 {
                                0
                            } else {
                                self.selected_story_index + 1
                            };
                    }
                    KeyCode::Up => {
                        self.selected_story_index = if self.selected_story_index == 0 {
                            self.stories.len() - 1
                        } else {
                            self.selected_story_index - 1
                        };
                    }
                    KeyCode::Enter => {
                        if let Some(story) = self.stories.get(self.selected_story_index) {
                            if let Err(e) = open::that(story.url.as_str()) {
                                eprintln!("Error opening url: {}", e);
                            }
                        }
                    }
                    KeyCode::Right => {
                        // breaks after page 5
                        if self.page < 5 {
                            self.page += 1;
                            self.stories = self
                                .client
                                .get(format!("https://lobste.rs/newest/page/{}.json", self.page))
                                .send()?
                                .json()?;
                        }
                    }
                    KeyCode::Left => {
                        if self.page > 1 {
                            self.page -= 1;
                            self.stories = self
                                .client
                                .get(format!("https://lobste.rs/newest/page/{}.json", self.page))
                                .send()?
                                .json()?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw_stories<B: Backend>(f: &mut Frame<B>, stories: &Stories, index: usize, page: usize) {
        let items: Vec<ListItem> = stories
            .iter()
            .enumerate()
            .map(|(i, s)| (s, i == index))
            .map(StoryWidget::new)
            .map(Into::into)
            .collect();

        let layout = tui::layout::Layout::default()
            .constraints(
                [
                    tui::layout::Constraint::Percentage(30),
                    tui::layout::Constraint::Percentage(65),
                    tui::layout::Constraint::Percentage(5),
                ]
                .as_ref(),
            )
            .margin(1)
            .split(f.size());

        let title = Paragraph::new(text::Text::raw(BANNER));
        f.render_widget(title, layout[0]);

        let items = List::new(items).block(Block::default());
        f.render_widget(items, layout[1]);

        let help = Paragraph::new(vec![
            vec![
                text::Span::styled(
                    format!("{} stories ", stories.len()),
                    Style::default()
                        .fg(tui::style::Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                "on page ".into(),
                text::Span::styled(
                    page.to_string(),
                    Style::default()
                        .fg(tui::style::Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
            ]
            .into(),
            "↑/↓: Navigate, Enter: Open in browser, q: Quit, ←/→: Navigate pages".into(),
        ]);
        f.render_widget(help, layout[2]);
    }
}

fn reset_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    println!("Shutting down...");

    disable_raw_mode().expect("Could not disable raw mode");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .expect("Could not leave alternate screen");
    terminal.show_cursor().expect("Could not show cursor");

    println!("Goodbye!");
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut app = App::init()?;
    app.run().map_err(|e| {
        reset_terminal(&mut app.terminal);
        e
    })?;

    reset_terminal(&mut app.terminal);
    Ok(())
}
