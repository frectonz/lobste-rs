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
    pub karma: i64,
    #[serde(rename = "avatar_url")]
    pub avatar_url: String,
    #[serde(rename = "invited_by_user")]
    pub invited_by_user: String,
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
}

impl App {
    fn init() -> Result<Self> {
        let client = Client::builder().build()?;
        let terminal = App::init_screen()?;

        let mut stories: Stories = client.get("https://lobste.rs/newest.json").send()?.json()?;
        stories.sort_by(|a, b| b.score.cmp(&a.score));

        Ok(Self {
            client,
            stories,
            terminal,
            selected_story_index: 0,
        })
    }

    fn init_screen() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Terminal::new(CrosstermBackend::new(io::stdout()))?)
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.terminal
                .draw(|f| Self::draw_stories(f, &self.stories, self.selected_story_index))?;

            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                } else if KeyCode::Down == key.code {
                    self.selected_story_index =
                        if self.selected_story_index == self.stories.len() - 1 {
                            0
                        } else {
                            self.selected_story_index + 1
                        };
                } else if KeyCode::Up == key.code {
                    self.selected_story_index = if self.selected_story_index == 0 {
                        self.stories.len() - 1
                    } else {
                        self.selected_story_index - 1
                    };
                } else if KeyCode::Enter == key.code {
                    if let Some(story) = self.stories.get(self.selected_story_index) {
                        if let Err(e) = open::that(story.url.as_str()) {
                            eprintln!("Error opening url: {}", e);
                        }
                    }
                } else if KeyCode::Char('r') == key.code {
                    self.stories = self
                        .client
                        .get("https://lobste.rs/newest.json")
                        .send()?
                        .json()?;
                    self.stories.sort_by(|a, b| b.score.cmp(&a.score));
                }
            }
        }
    }

    fn draw_stories<B: Backend>(f: &mut Frame<B>, stories: &Stories, index: usize) {
        let items: Vec<ListItem> = stories
            .iter()
            .enumerate()
            .map(|(i, s)| (s, i == index))
            .map(|(s, selected)| {
                let title = text::Span::styled(
                    &s.title,
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(if selected {
                            tui::style::Color::Green
                        } else {
                            tui::style::Color::White
                        }),
                );
                let url = text::Span::styled(&s.url, Style::default().fg(tui::style::Color::Blue));

                let span = text::Spans::from(vec![
                    if selected { "► " } else { "  " }.into(),
                    format!("⧋ {:3}   ", s.score).into(),
                    title,
                    " ".into(),
                    url,
                ]);

                ListItem::new(span)
            })
            .collect();

        let layout = tui::layout::Layout::default()
            .constraints(
                [
                    tui::layout::Constraint::Percentage(30),
                    tui::layout::Constraint::Percentage(60),
                    tui::layout::Constraint::Length(3),
                ]
                .as_ref(),
            )
            .margin(1)
            .split(f.size());

        let title = Paragraph::new(text::Text::raw(BANNER));
        f.render_widget(title, layout[0]);

        let items = List::new(items).block(Block::default());
        f.render_widget(items, layout[1]);

        let help = Paragraph::new(text::Text::raw(
            "↑/↓: Navigate, Enter: Open in browser, q: Quit, r: Refresh",
        ));
        f.render_widget(help, layout[2]);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        println!("Shutting down...");

        disable_raw_mode().expect("Could not disable raw mode");
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .expect("Could not leave alternate screen");
        self.terminal.show_cursor().expect("Could not show cursor");

        println!("Goodbye!");
    }
}

fn main() -> Result<()> {
    let mut app = App::init()?;
    app.run()?;
    Ok(())
}
