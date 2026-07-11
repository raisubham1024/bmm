use super::common::*;
use super::model::{MessageKind, Model};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListDirection, ListItem, Padding, Paragraph},
};

const HELP_CONTENTS: &str = include_str!("static/help.txt");

pub(crate) fn view(model: &mut Model, frame: &mut Frame) {
    if model.terminal_too_small {
        render_terminal_too_small_view(&model.terminal_dimensions, frame);
        return;
    }

    match model.active_pane {
        ActivePane::List => {
            if model.initial {
                render_initial_view(model, frame, false);
            } else {
                render_list_view(model, frame, false);
            }
        }
        ActivePane::Help => render_help_view(model, frame),
        ActivePane::SearchInput => {
            if model.initial {
                render_initial_view(model, frame, true);
            } else {
                render_list_view(model, frame, true);
            }
        }
        ActivePane::TagsList => render_tag_list_view(model, frame, false),
        ActivePane::TagSearchInput => render_tag_list_view(model, frame, true),
        ActivePane::EditBookmark => render_edit_bookmark_view(model, frame),
        ActivePane::Notes => render_notes_view(model, frame),
        ActivePane::Confirm => render_confirm_view(model, frame),
    }
}

fn render_terminal_too_small_view(dimensions: &TerminalDimensions, frame: &mut Frame) {
    let message = format!(
        r#"
Terminal size too small:
  Width = {} Height = {}

Minimum dimensions needed:
  Width = {} Height = {}

Press (q/<ctrl+c>/<esc> to exit)
"#,
        dimensions.width, dimensions.height, MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
    );

    let p = Paragraph::new(message)
        .block(Block::bordered())
        .style(Style::new().fg(PRIMARY_COLOR))
        .alignment(Alignment::Center);

    frame.render_widget(p, frame.area());
}

fn render_banner(terminal_height: u16, frame: &mut Frame, chunk: Rect) {
    let banner = r#"
bbbbbbb                                                                
b:::::b                                                                
b:::::b                                                                
b:::::b                                                                
b:::::b                                                                
b:::::bbbbbbbbb         mmmmmmm    mmmmmmm        mmmmmmm    mmmmmmm   
b::::::::::::::bb     mm:::::::m  m:::::::mm    mm:::::::m  m:::::::mm 
b::::::::::::::::b   m::::::::::mm::::::::::m  m::::::::::mm::::::::::m
b:::::bbbbb:::::::b  m::::::::::::::::::::::m  m::::::::::::::::::::::m
b:::::b    b::::::b  m:::::mmm::::::mmm:::::m  m:::::mmm::::::mmm:::::m
b:::::b     b:::::b  m::::m   m::::m   m::::m  m::::m   m::::m   m::::m
b:::::b     b:::::b  m::::m   m::::m   m::::m  m::::m   m::::m   m::::m
b:::::b     b:::::b  m::::m   m::::m   m::::m  m::::m   m::::m   m::::m
b:::::bbbbbb::::::b  m::::m   m::::m   m::::m  m::::m   m::::m   m::::m
b::::::::::::::::b   m::::m   m::::m   m::::m  m::::m   m::::m   m::::m
b:::::::::::::::b    m::::m   m::::m   m::::m  m::::m   m::::m   m::::m
bbbbbbbbbbbbbbbb     mmmmmm   mmmmmm   mmmmmm  mmmmmm   mmmmmm   mmmmmm


type your search query and press enter
"#;

    let top_padding = if terminal_height > 26 {
        ((terminal_height - 22) / 2) - 2
    } else {
        0
    };

    let p = Paragraph::new(banner)
        .style(Style::new().fg(PRIMARY_COLOR))
        .block(Block::new().padding(Padding::new(0, 0, top_padding, 0)))
        .alignment(Alignment::Center);

    frame.render_widget(p, chunk);
}

fn render_header(model: &Model, frame: &mut Frame, chunk: Rect) {
    let mut header_components = Vec::new();

    match model.active_pane {
        ActivePane::List | ActivePane::SearchInput => {
            if model.bookmark_items.items.is_empty() {
                header_components.push(Span::styled(
                    " no bookmarks ",
                    Style::new().bold().bg(PRIMARY_COLOR).fg(FG_COLOR),
                ));
            } else {
                header_components.push(Span::styled(
                    " bookmarks ",
                    Style::new().bold().bg(PRIMARY_COLOR).fg(FG_COLOR),
                ));
                header_components.push(Span::from(" "));
                header_components.push(Span::styled(
                    format!("({})", model.bookmark_items.items.len()),
                    Style::new().fg(COLOR_THREE),
                ));
            }
        }
        ActivePane::Help => {
            header_components.push(Span::styled(
                " help ",
                Style::new().bold().bg(HELP_COLOR).fg(FG_COLOR),
            ));
        }
        ActivePane::TagsList | ActivePane::TagSearchInput => {
            if model.tag_items.items.is_empty() {
                header_components.push(Span::styled(
                    " no tags ",
                    Style::new().bold().bg(TAGS_COLOR).fg(FG_COLOR),
                ));
            } else {
                header_components.push(Span::styled(
                    " tags ",
                    Style::new().bold().bg(TAGS_COLOR).fg(FG_COLOR),
                ));
                header_components.push(Span::from(" "));
                header_components.push(Span::styled(
                    format!("({})", model.tag_items.items.len()),
                    Style::new().fg(COLOR_THREE),
                ));
            }
        }
        ActivePane::EditBookmark => {
            header_components.push(Span::styled(
                " editing bookmark ",
                Style::new().bold().bg(COLOR_TWO).fg(FG_COLOR),
            ));
        }
        ActivePane::Notes => {
            header_components.push(Span::styled(
                " note (hidden) ",
                Style::new().bold().bg(COLOR_TWO).fg(FG_COLOR),
            ));
        }
        ActivePane::Confirm => {
            header_components.push(Span::styled(
                " confirm ",
                Style::new().bold().bg(PRIMARY_COLOR).fg(FG_COLOR),
            ));
        }
    }

    let header_text = Line::from(header_components);

    let header =
        Paragraph::new(header_text).block(Block::default().padding(Padding::new(2, 0, 1, 0)));

    frame.render_widget(&header, chunk);
}

fn render_status_line(model: &Model, frame: &mut Frame, chunk: Rect) {
    let mut status_bar_lines = vec![Span::styled(
        TITLE,
        Style::new().bold().bg(PRIMARY_COLOR).fg(FG_COLOR),
    )];

    if model.debug {
        status_bar_lines.push(Span::from(format!(
            " [render counter: {}]",
            model.render_counter
        )));
        status_bar_lines.push(Span::from(format!(
            " [event counter: {}]",
            model.event_counter
        )));

        status_bar_lines.push(Span::from(format!(
            " [dimensions: {}x{}] ",
            model.terminal_dimensions.width, model.terminal_dimensions.height
        )));
    }

    if let Some(msg) = &model.user_message {
        let span = match msg.kind {
            MessageKind::Info => Span::styled(
                format!(" {}", msg.value),
                Style::new().fg(INFO_MESSAGE_COLOR),
            ),
            MessageKind::Error => Span::styled(
                format!(" {}", msg.value),
                Style::new().fg(ERROR_MESSAGE_COLOR),
            ),
        };

        status_bar_lines.push(span);
    }

    let status_bar_text = Line::from(status_bar_lines);

    let status_bar = Paragraph::new(status_bar_text).block(Block::default());

    frame.render_widget(&status_bar, chunk);
}

fn render_search_input(model: &Model, frame: &mut Frame, chunk: Rect) {
    // keep 2 for borders and 1 for cursor
    let width = chunk.width.max(3) - 3;
    let scroll = model.search_input.visual_scroll(width as usize);

    let input = Paragraph::new(model.search_input.value())
        .style(Style::default().fg(COLOR_THREE))
        .scroll((0, scroll as u16))
        .block(
            Block::bordered()
                .title(" search query? ")
                .title_style(Style::new().bold().bg(COLOR_THREE).fg(FG_COLOR)),
        );
    frame.render_widget(input, chunk);

    let cursor_x = model.search_input.visual_cursor().max(scroll) - scroll + 1;
    frame.set_cursor_position((chunk.x + cursor_x as u16, chunk.y + 1));
}

fn render_tag_search_input(model: &Model, frame: &mut Frame, chunk: Rect) {
    // keep 2 for borders and 1 for cursor
    let width = chunk.width.max(3) - 3;
    let scroll = model.tag_search_input.visual_scroll(width as usize);

    let input = Paragraph::new(model.tag_search_input.value())
        .style(Style::default().fg(TAGS_COLOR))
        .scroll((0, scroll as u16))
        .block(
            Block::bordered()
                .title(" search tags? ")
                .title_style(Style::new().bold().bg(TAGS_COLOR).fg(FG_COLOR)),
        );
    frame.render_widget(input, chunk);

    let cursor_x = model.tag_search_input.visual_cursor().max(scroll) - scroll + 1;
    frame.set_cursor_position((chunk.x + cursor_x as u16, chunk.y + 1));
}

fn render_bookmarks_list(model: &mut Model, frame: &mut Frame, chunk: Rect) {
    let items: Vec<ListItem> = model
        .bookmark_items
        .items
        .iter()
        .map(ListItem::from)
        .collect();

    let list = List::new(items)
        .block(Block::new().padding(Padding::new(0, 0, 1, 1)))
        .style(Style::new().white())
        .highlight_symbol("> ")
        .repeat_highlight_symbol(true)
        .highlight_style(Style::new().fg(PRIMARY_COLOR))
        .direction(ListDirection::TopToBottom);

    frame.render_stateful_widget(&list, chunk, &mut model.bookmark_items.state);
}

fn render_tag_list(model: &mut Model, frame: &mut Frame, chunk: Rect) {
    let items: Vec<ListItem> = model.tag_items.items.iter().map(ListItem::from).collect();

    let list = List::new(items)
        .block(Block::new().padding(Padding::new(0, 0, 1, 1)))
        .style(Style::new().white())
        .highlight_symbol("> ")
        .repeat_highlight_symbol(true)
        .highlight_style(Style::new().fg(TAGS_COLOR))
        .direction(ListDirection::TopToBottom);

    frame.render_stateful_widget(&list, chunk, &mut model.tag_items.state);
}

fn render_bookmarks_details(model: &Model, frame: &mut Frame, chunk: Rect) {
    let maybe_selected = model.bookmark_items.state.selected();

    if let Some(selected) = maybe_selected {
        let maybe_bookmark_item = model.bookmark_items.items.get(selected);
        if let Some(bookmark_item) = maybe_bookmark_item {
            let details = format!(
                r#"URI   : {}
Title : {}
Tags  : {}
"#,
                bookmark_item.bookmark.uri,
                bookmark_item
                    .bookmark
                    .title
                    .as_deref()
                    .unwrap_or("<NOT SET>"),
                bookmark_item
                    .bookmark
                    .tags
                    .as_deref()
                    .unwrap_or("<NOT SET>")
            );
            let details = Paragraph::new(details)
                .block(
                    Block::bordered()
                        .border_style(Style::default().fg(COLOR_TWO))
                        .title_style(Style::new().bold().bg(COLOR_TWO).fg(FG_COLOR))
                        .title(" details ")
                        .padding(Padding::new(1, 0, 1, 0)),
                )
                .style(Style::new().white().on_black())
                .alignment(Alignment::Left);

            frame.render_widget(&details, chunk);
        };
    }
}

fn render_tag_details(model: &Model, frame: &mut Frame, chunk: Rect) {
    let maybe_selected = model.tag_items.state.selected();

    if let Some(selected) = maybe_selected {
        let maybe_tag_item = model.tag_items.items.get(selected);
        if let Some(tag_with_stats) = maybe_tag_item {
            let details = format!(r#"Number of bookmarks : {}"#, tag_with_stats.num_bookmarks);
            let details = Paragraph::new(details)
                .block(
                    Block::bordered()
                        .border_style(Style::default().fg(COLOR_TWO))
                        .title_style(Style::new().bold().bg(COLOR_TWO).fg(FG_COLOR))
                        .title(" details ")
                        .padding(Padding::new(1, 0, 1, 1)),
                )
                .style(Style::new().white().on_black())
                .alignment(Alignment::Left);

            frame.render_widget(&details, chunk);
        };
    }
}

fn render_initial_view(model: &mut Model, frame: &mut Frame, search: bool) {
    match search {
        true => {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(20),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(frame.area());

            render_banner(model.terminal_dimensions.height, frame, layout[0]);
            render_search_input(model, frame, layout[1]);
            render_status_line(model, frame, layout[2]);
        }
        false => {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(vec![Constraint::Min(21), Constraint::Length(1)])
                .split(frame.area());

            render_banner(model.terminal_dimensions.height, frame, layout[0]);
            render_status_line(model, frame, layout[1]);
        }
    }
}

fn render_list_view(model: &mut Model, frame: &mut Frame, search: bool) {
    match model.bookmark_items.items.len() {
        0 => match search {
            true => {
                let layout = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(2),
                        Constraint::Min(18),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ])
                    .split(frame.area());

                render_header(model, frame, layout[0]);
                render_bookmarks_list(model, frame, layout[1]);
                render_search_input(model, frame, layout[2]);
                render_status_line(model, frame, layout[3]);
            }
            false => {
                let layout = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(2),
                        Constraint::Min(21),
                        Constraint::Length(1),
                    ])
                    .split(frame.area());

                render_header(model, frame, layout[0]);
                render_bookmarks_list(model, frame, layout[1]);
                render_status_line(model, frame, layout[2]);
            }
        },
        _ => match search {
            true => {
                let layout = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(2),
                        Constraint::Min(11),
                        Constraint::Length(7),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ])
                    .split(frame.area());

                render_header(model, frame, layout[0]);
                render_bookmarks_list(model, frame, layout[1]);
                render_bookmarks_details(model, frame, layout[2]);
                render_search_input(model, frame, layout[3]);
                render_status_line(model, frame, layout[4]);
            }
            false => {
                let layout = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(2),
                        Constraint::Min(14),
                        Constraint::Length(7),
                        Constraint::Length(1),
                    ])
                    .split(frame.area());

                render_header(model, frame, layout[0]);
                render_bookmarks_list(model, frame, layout[1]);
                render_bookmarks_details(model, frame, layout[2]);
                render_status_line(model, frame, layout[3]);
            }
        },
    }
}

fn render_tag_list_view(model: &mut Model, frame: &mut Frame, search: bool) {
    match search {
        true => {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(2),
                    Constraint::Min(12),
                    Constraint::Length(5),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(frame.area());

            render_header(model, frame, layout[0]);
            render_tag_list(model, frame, layout[1]);
            render_tag_details(model, frame, layout[2]);
            render_tag_search_input(model, frame, layout[3]);
            render_status_line(model, frame, layout[4]);
        }
        false => {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(2),
                    Constraint::Min(16),
                    Constraint::Length(5),
                    Constraint::Length(1),
                ])
                .split(frame.area());

            render_header(model, frame, layout[0]);
            render_tag_list(model, frame, layout[1]);
            render_tag_details(model, frame, layout[2]);
            render_status_line(model, frame, layout[3]);
        }
    }
}

fn render_edit_bookmark_view(model: &mut Model, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(9),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(model, frame, layout[0]);
    render_bookmarks_list(model, frame, layout[1]);
    render_edit_bookmark_fields(model, frame, layout[2]);
    render_status_line(model, frame, layout[3]);
}

fn render_edit_bookmark_fields(model: &Model, frame: &mut Frame, chunk: Rect) {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(chunk);

    render_edit_uri_field(model, frame, layout[0]);
    render_edit_title_field(model, frame, layout[1]);
    render_edit_tags_field(model, frame, layout[2]);
}

fn render_edit_uri_field(model: &Model, frame: &mut Frame, chunk: Rect) {
    let p = Paragraph::new(model.edit_uri.as_str())
        .style(Style::default().fg(COLOR_TWO))
        .block(
            Block::bordered()
                .border_style(Style::default().fg(COLOR_TWO))
                .title(" URI (read-only) ")
                .title_style(Style::new().bold().bg(COLOR_TWO).fg(FG_COLOR)),
        );
    frame.render_widget(p, chunk);
}

fn render_edit_title_field(model: &Model, frame: &mut Frame, chunk: Rect) {
    let focused = model.edit_focus == EditField::Title;
    let color = if focused { PRIMARY_COLOR } else { COLOR_TWO };

    // keep 2 for borders and 1 for cursor
    let width = chunk.width.max(3) - 3;
    let scroll = model.edit_title_input.visual_scroll(width as usize);

    let title = if focused {
        " Title (Tab: switch, Ctrl+s: save, Esc: cancel) "
    } else {
        " Title "
    };

    let p = Paragraph::new(model.edit_title_input.value())
        .style(Style::default().fg(color))
        .scroll((0, scroll as u16))
        .block(
            Block::bordered()
                .border_style(Style::default().fg(color))
                .title(title)
                .title_style(Style::new().bold().bg(color).fg(FG_COLOR)),
        );
    frame.render_widget(p, chunk);

    if focused {
        let cursor_x = model.edit_title_input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((chunk.x + cursor_x as u16, chunk.y + 1));
    }
}

fn render_edit_tags_field(model: &Model, frame: &mut Frame, chunk: Rect) {
    let focused = model.edit_focus == EditField::Tags;
    let color = if focused { PRIMARY_COLOR } else { COLOR_TWO };

    // keep 2 for borders and 1 for cursor
    let width = chunk.width.max(3) - 3;
    let scroll = model.edit_tags_input.visual_scroll(width as usize);

    let title = if focused {
        " Tags, comma separated (Tab: switch, Ctrl+s: save, Esc: cancel) "
    } else {
        " Tags, comma separated "
    };

    let p = Paragraph::new(model.edit_tags_input.value())
        .style(Style::default().fg(color))
        .scroll((0, scroll as u16))
        .block(
            Block::bordered()
                .border_style(Style::default().fg(color))
                .title(title)
                .title_style(Style::new().bold().bg(color).fg(FG_COLOR)),
        );
    frame.render_widget(p, chunk);

    if focused {
        let cursor_x = model.edit_tags_input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((chunk.x + cursor_x as u16, chunk.y + 1));
    }
}

fn render_notes_view(model: &mut Model, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(model, frame, layout[0]);
    render_bookmarks_list(model, frame, layout[1]);
    render_note_field(model, frame, layout[2]);
    render_status_line(model, frame, layout[3]);
}

fn render_note_field(model: &Model, frame: &mut Frame, chunk: Rect) {
    // keep 2 for borders and 1 for cursor
    let width = chunk.width.max(3) - 3;
    let scroll = model.note_input.visual_scroll(width as usize);

    let title = format!(
        " note for {} (Ctrl+s: save, Esc: cancel) ",
        model.note_uri
    );

    let p = Paragraph::new(model.note_input.value())
        .style(Style::default().fg(PRIMARY_COLOR))
        .scroll((0, scroll as u16))
        .block(
            Block::bordered()
                .border_style(Style::default().fg(PRIMARY_COLOR))
                .title(title)
                .title_style(Style::new().bold().bg(PRIMARY_COLOR).fg(FG_COLOR)),
        );
    frame.render_widget(p, chunk);

    let cursor_x = model.note_input.visual_cursor().max(scroll) - scroll + 1;
    frame.set_cursor_position((chunk.x + cursor_x as u16, chunk.y + 1));
}

fn render_confirm_view(model: &Model, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(model, frame, layout[0]);

    let message = model.confirm_message();
    let text = format!("{message}\n\n[y] yes            [n] no");

    let p = Paragraph::new(text)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(PRIMARY_COLOR))
                .title_style(Style::new().bold().bg(PRIMARY_COLOR).fg(FG_COLOR))
                .title(" are you sure? ")
                .padding(Padding::new(2, 2, 2, 0)),
        )
        .style(Style::new().white().on_black())
        .alignment(Alignment::Center);

    frame.render_widget(&p, layout[1]);
    render_status_line(model, frame, layout[2]);
}

fn render_help_view(model: &mut Model, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![
            Constraint::Length(2),
            Constraint::Min(21),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(model, frame, layout[0]);
    let lines: Vec<Line<'_>> = HELP_CONTENTS.lines().map(Line::from).collect();

    let p = Paragraph::new(lines)
        .block(Block::new().padding(Padding::new(2, 0, 1, 0)))
        .style(Style::new().white())
        .alignment(Alignment::Left);

    frame.render_widget(p, layout[1]);
    render_status_line(model, frame, layout[2]);
}
