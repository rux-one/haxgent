mod logger;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{collections::HashMap, io, thread::current};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use chrono::{DateTime, Utc, TimeZone};

use crate::logger::Logger;

enum AgentState {
    Idle,
    Thinking,
    Working,
}

struct AgentConfig {
    host: String,
}

struct Agent {
    state: AgentState,
    config: AgentConfig,
}

impl Agent {
    fn new() -> Self {
        Self {
            state: AgentState::Idle,
            config: AgentConfig {
                host: String::from("127.0.0.1"),
            },
        }
    }

    fn set_state(&mut self, state: AgentState) {
        self.state = state;
    }

    fn get_state(&self) -> &AgentState {
        &self.state
    }

    fn set_host(&mut self, host: String) {
        self.config.host = host;
    }

    fn get_host(&self) -> &String {
        &self.config.host
    }

    fn poke(&mut self) {
        self.set_state(AgentState::Thinking);
    }
}

struct Commands {
    commands: HashMap<String, String>,
    current_command: String,
    current_command_args: Vec<String>,
}

impl Commands {
    fn new() -> Self {
        Self {
            commands: HashMap::new(),
            current_command: String::new(),
            current_command_args: vec![],
        }
    }

    fn add_command(&mut self, command: String, description: String) {
        self.commands.insert(command, description);
    }

    fn setup_commands(&self) -> HashMap<String, String> {
        let mut commands = HashMap::new();
        commands.insert("sethost".to_string(), "Set host".to_string());
        commands.insert("help".to_string(), "Available commands: hello, quit, sethost".to_string());
        commands.insert("quit".to_string(), "Goodbye!".to_string());
        commands
    }

    fn capture_command(&mut self, input: &str) {
        if input.is_empty() {
            return;
        }

        let cmd = input.split(" ").collect::<Vec<&str>>()[0];
        let args = input.split(" ").collect::<Vec<&str>>()[1..].join(" ");

        self.current_command = String::from(cmd);
        self.current_command_args = args.split(" ").map(|s| s.to_string()).collect();
    }

    fn get_current_command(&self) -> &String {
        &self.current_command
    }

    fn get_current_command_args(&self) -> &Vec<String> {
        &self.current_command_args
    }

    fn reset_command(&mut self) {
        self.current_command = String::new();
        self.current_command_args = vec![];
    }
}

struct App {
    input: String,
    agent: Agent,
    commands: Commands,
    log: Logger,
    log_state: ListState,
}

impl App {
    fn new() -> Self {
        let mut log_state = ListState::default();
        log_state.select(Some(0));
        Self {
            input: String::new(),
            agent: Agent::new(),
            commands: Commands::new(),
            log: Logger::new(),
            log_state,
        }
    }

    fn next_log(&mut self) {
        let logs = self.log.get_logs();
        if logs.is_empty() {
            return;
        }
        let i = match self.log_state.selected() {
            Some(i) => {
                if i >= logs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.log_state.select(Some(i));
    }

    fn previous_log(&mut self) {
        let logs = self.log.get_logs();
        if logs.is_empty() {
            return;
        }
        let i = match self.log_state.selected() {
            Some(i) => {
                if i == 0 {
                    logs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.log_state.select(Some(i));
    }
}

fn main() -> Result<(), io::Error> {
    let mut app = App::new();

    app.log.add(
        String::from("Welcome!"),
        String::from("The default host has been set to 127.0.0.1"),
    );

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;
    
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .constraints([Constraint::Max(3), Constraint::Max(5), Constraint::Max(1)].as_ref())
                .split(size);

            app.commands.capture_command(&app.input);

            let logs = app.log.get_logs();
            let log_items: Vec<ListItem> = logs
                .iter()
                .map(|entry| {
                    let dt = Utc.timestamp_opt(entry.created_at as i64, 0).unwrap();
                    let content = format!("{} -- {}", dt.format("%Y-%m-%d %H:%M:%S"), entry.summary);
                    ListItem::new(content)
                })
                .collect();

            let log_list = List::new(log_items)
                .block(Block::default().borders(Borders::ALL).title(" -- log --"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            let details_content = if let Some(selected) = app.log_state.selected() {
                if let Some(entry) = logs.get(selected) {
                    format!("Time: {}\nSummary: {}\n\nDetails:\n{}", 
                        Utc.timestamp_opt(entry.created_at as i64, 0).unwrap().format("%Y-%m-%d %H:%M:%S"),
                        entry.summary,
                        entry.details)
                } else {
                    String::from("No log selected")
                }
            } else {
                String::from("No log selected")
            };

            let log_details_widget = Paragraph::new(details_content)
                .block(Block::default().borders(Borders::ALL).title(" -- details --"));
            
            let log_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(chunks[2]);

            f.render_stateful_widget(log_list, log_chunks[0], &mut app.log_state);
            f.render_widget(log_details_widget, log_chunks[1]);

            let settings = format!("Target host: {}", app.agent.get_host());
            let settings_widget = Paragraph::new(settings)
                .block(Block::default().borders(Borders::ALL).title(" -- settings -- "));
            f.render_widget(settings_widget, chunks[1]);
            
            let input_widget = Paragraph::new(app.input.as_ref() as &str)
                .block(Block::default().borders(Borders::ALL).title(" -- command --"));
            f.render_widget(input_widget, chunks[0]);
        })?;

        // Handle input
        if crossterm::event::poll(std::time::Duration::from_millis(500))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {

                match key.code {
                    crossterm::event::KeyCode::Enter => {
                        // Execute command on Enter key
                        match app.commands.get_current_command().as_str() {
                            "sethost" => {
                                app.agent.set_host(app.commands.get_current_command_args()[0].clone());
                                app.log.add(
                                    format!("Now looking at host: {}", app.agent.get_host()),
                                    format!("Host changed to {}", app.agent.get_host())
                                );

                                app.agent.poke();
                            },
                            "exit" => break,
                            _ => {}
                        }

                        app.input.clear();  // Clear input after execution
                    }
                    crossterm::event::KeyCode::Esc => break,  // Exit on Esc key
                    crossterm::event::KeyCode::Char(c) => {
                        app.input.push(c);  // Add character to input
                    }
                    crossterm::event::KeyCode::Backspace => {
                        app.input.pop();  // Remove last character on Backspace
                    }
                    crossterm::event::KeyCode::Up => app.previous_log(),
                    crossterm::event::KeyCode::Down => app.next_log(),
                    _ => {}
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
