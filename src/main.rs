use std::io;

use color_eyre::{eyre::eyre, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use reqwest::blocking::Client;
use tui::{
    backend::{Backend, CrosstermBackend},
    style::{Modifier, Style},
    text,
    widgets::{Block, List, ListItem, Paragraph},
    Frame, Terminal,
};

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

pub struct Story(serde_json::Value);

impl Story {
    fn url(&self) -> Option<&str> {
        let serde_json::Value::Object(ref story) = self.0 else {
            return None;
        };

        let serde_json::Value::String(url) = story.get("url")? else {
            return None;
        };
        let serde_json::Value::String(short_id_url) = story.get("short_id_url")? else {
            return None;
        };
        if url.is_empty() {
            Some(short_id_url)
        } else {
            Some(url)
        }
    }

    fn url_span(&self) -> Option<text::Span> {
        self.url().map(|url| {
            text::Span::styled(
                url,
                Style::default()
                    .fg(tui::style::Color::Blue)
                    .add_modifier(tui::style::Modifier::UNDERLINED),
            )
        })
    }

    fn title(&self) -> Option<&str> {
        let serde_json::Value::Object(ref story) = self.0 else {
            return None;
        };

        let serde_json::Value::String(title) = story.get("title")? else {
            return None;
        };

        Some(title)
    }

    fn title_span(&self, selected: bool) -> Option<text::Span> {
        self.title().map(|title| {
            text::Span::styled(
                title,
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(if selected {
                        tui::style::Color::Green
                    } else {
                        tui::style::Color::White
                    }),
            )
        })
    }

    fn score(&self) -> Option<i64> {
        let serde_json::Value::Object(ref story) = self.0 else {
            return None;
        };

        let serde_json::Value::Number(score) = story.get("score")? else {
            return None;
        };

        score.as_i64()
    }

    fn score_span(&self) -> Option<text::Span> {
        self.score().map(|score| {
            text::Span::styled(
                format!("⧋ {: <4}", score),
                Style::default().fg(tui::style::Color::Yellow),
            )
        })
    }
}

struct StoryWidget<'a> {
    story: &'a Story,
    selected: bool,
}

impl<'a> StoryWidget<'a> {
    fn new(story: &'a Story, selected: bool) -> Self {
        Self { story, selected }
    }

    fn to_item(&self) -> Option<ListItem<'a>> {
        let selected_indicator = if self.selected { "► " } else { "  " };

        let span = text::Spans::from(vec![
            selected_indicator.into(),
            self.story.score_span()?,
            " ".into(),
            self.story.title_span(self.selected)?,
            " ".into(),
            self.story.url_span()?,
        ]);

        Some(ListItem::new(span))
    }
}

fn get_stories(stories: serde_json::Value) -> Option<Vec<Story>> {
    let serde_json::Value::Array(stories) = stories else {
        return None;
    };

    Some(stories.into_iter().map(Story).collect())
}

struct App {
    client: Client,
    stories: Vec<Story>,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    selected_story_index: usize,
    page: usize,
}

impl App {
    fn init() -> Result<Self> {
        let client = Client::builder().build()?;
        let mut terminal = App::init_screen()?;

        let stories = client
            .get("https://lobste.rs/newest.json")
            .send()?
            .json()
            .map_err(|e| {
                reset_terminal(&mut terminal);
                e
            })?;
        let stories = get_stories(stories).ok_or(eyre!("couldn't find stories"))?;

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
                            if story.url().and_then(|url| open::that(url).ok()).is_none() {
                                eprintln!("Error opening url");
                            }
                        }
                    }
                    KeyCode::Right => {
                        // breaks after page 5
                        if self.page < 5 {
                            self.page += 1;
                            let stories = self
                                .client
                                .get(format!("https://lobste.rs/newest/page/{}.json", self.page))
                                .send()?
                                .json()?;

                            self.stories =
                                get_stories(stories).ok_or(eyre!("couldn't find stories"))?;
                        }
                    }
                    KeyCode::Left => {
                        if self.page > 1 {
                            self.page -= 1;
                            let stories = self
                                .client
                                .get(format!("https://lobste.rs/newest/page/{}.json", self.page))
                                .send()?
                                .json()?;

                            self.stories =
                                get_stories(stories).ok_or(eyre!("couldn't find stories"))?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw_stories<B: Backend>(f: &mut Frame<B>, stories: &[Story], index: usize, page: usize) {
        let items: Vec<ListItem> = stories
            .iter()
            .enumerate()
            .map(|(i, s)| StoryWidget::new(s, i == index))
            .filter_map(|s| s.to_item())
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
