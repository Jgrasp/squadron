# Changelog

## 0.38.1 · 2026-11-09

### Fixes

- The context breakdown shows as soon as the gauge opens, even when the agent has not received a message since Squadron started.

## 0.38.0 · 2026-11-09

### Context

- The context gauge opens a breakdown of everything in the context, at startup and from the conversation: each plugin's share counted by Claude Code, a plugin skill's instructions apart from your own messages, and the compaction instructions. Settings have a tab for it, and the plugins pill is gone.
- A plugin skill you launch yourself with its slash command counts in the agent's plugin usage, at the cost the plugin declares for it.

### Extensions

- An extension can start Claude Code with options of its own, like --chrome, once you allow them.

### Pull requests

- A merged pull request opens again from the review pill, with its changes and your draft remarks to post.

### Conversation

- Opening a file card's content glides the conversation until the card sits just under the agent header.

### Fixes

- An agent no longer stops with "mcp__squadron__permission not found" after Squadron was opened a second time.
- Plugin costs load without opening the Plugins settings first.
- The message field no longer keeps a blank band under its placeholder after a suggestion appears, and drops back to its height when the suggestion changes while it is not focused.

## 0.37.0 · 2026-11-09

### Plugins

- Settings have a Plugins tab: install Claude Code plugins and turn each on for every project or only the ones you tick.
- Each agent shows the plugins it loaded and what they cost, and the conversation marks where a plugin kicked in.

### Projects

- The Projects panel shows groups as tabs, sub-groups as chips and projects as cards with their open agents, questions, errors and done agents.
- It sorts by activity, attention, name, open agents or by hand, widens to several columns by dragging or double-clicking its edge, and the effect of a working project card (snake, live stripe or pulse) is chosen in Preferences.

### Message box

- A file or folder cited with @ shows as a chip, with an icon colored by file type, or for a folder its parent path and file count. Sent messages show the same chips.

### Conversation

- A large file opens fast in its file card: the preview shows its first 300 lines and loads the next ones as you scroll.
- File cards and attachments show an icon for the file type instead of the extension text.
- A file card's content unfolds downward with a slide, and the conversation glides just enough to show it.

### Tickets

- A Jira ticket opens in a full window: description, attachments and comments on the left, fields and links on the right, from the Kanban and from a conversation.

### Pull requests

- A comment or reply form slides open below its line, and the diff glides just enough to show it.
- Finish review sends to the agent the comments added for it as well as the review draft, and counts them.

### Agents

- An agent can rename itself, or another agent of its project, with the set_title tool.
- Every agent reads the project's DESIGN.md before an artifact, an html block or a style change, and an html artifact can link /__squadron/theme.css to get Squadron's colors, classes and fonts.

### Fixes

- Clicking Update looks again for the latest release, so a version published since the last check is the one installed.
- On Windows, a terminal or a ! command on a project under \\wsl$ opens in its WSL distro and folder.
- A pull request no longer stays stuck on fetching when its reviewed commit changes while it loads, or when two draft comments share a line.
- A review agent's second comment on a line replaces its first instead of adding a duplicate.
- The Plugins tab no longer stays on Loading when a plugin is installed in two scopes of the same project.

## 0.36.1 · 2026-11-09

### Fixes

- File cards in a reply stack one per line instead of sitting side by side.
- A new agent's first message field has the focus when the dialog opens, and the caret stays visible at the start of a line.
- The status bar shows Squadron's version on the left.
- The Jira ticket panel header stays on one line: type and sprint shorten, and View all and Open in Jira keep only their icon when the panel is narrow.

## 0.36.0 · 2026-11-06

### Extensions

- Squadron can be extended without rebuilding it. Extensions add buttons, menu entries, panels, colors and tools your agents can call, with an extension.json and, for their logic, Rust compiled to WebAssembly.
- Settings › Extensions, also in the Squadron menu, is a searchable catalog: install, update and uninstall in one click, from the official source or GitHub repositories you add, or load a folder you are writing. It reloads at every build.
- An extension asks before running a command or reaching a site. Its tokens stay in the system keyring and its panels run isolated.
- Write your own with the guide: https://github.com/Jgrasp/squadron-extensions/blob/main/docs/guide.md

### Message box

- The message fields are code editors: a fenced code block shows in a mono, colored block while you type, in the conversation and the new agent dialog.
- Pasted code goes into a fenced block with its language, taken from VS Code or guessed. Ctrl+Shift+V pastes raw.
- The warning about an old conversation takes compaction instructions, passed to /compact before the message is sent.

### Conversation

- A file path written as inline code in a reply gets its file card. File cards take the full width of the reply, and several in a row sit in columns.
- A single-question card has its own free answer field, and that field now reads as a field.
- Markdown tables keep their columns readable: no more letter-by-letter wrapping, and inline code in a cell stays on one line.
- Searching a long conversation no longer freezes Squadron at the first letter, and the find bar no longer reopens in the next conversation.

### Panels

- A click outside a panel in overlay mode closes it; side, bottom and full panels stay open.
- Panels slide smoothly, and a resize drag moves only the panel until you let go.
- The Jira ticket opened from the Kanban covers the board below its header.

### Fixes

- An agent's branch and pull request follow its worktree after a checkout or a switch.
- On Linux, a copy made right after coming back to the window replaces what was in the clipboard.

## 0.35.3 · 2026-11-05

### Fixes

- A Ctrl+click or Shift+click on a link in a conversation opens it once in the browser, instead of twice.

## 0.35.1 · 2026-11-05

### Fixes

- On Windows, the files Claude hands over in a conversation open again: their card no longer says the file is gone when Claude writes its path as /c/Users/… or C:\…
- When a file cannot be opened, its card's tooltip still shows its path.

## 0.35.0 · 2026-11-05

### New agent

- When the first message starts with /, the new agent dialog suggests skills with their arguments, like the message box.

### Conversation

- The conversation and the message box are centered on the same column: the message rail runs down beside the message box.
- Stopping an agent puts the cursor back in its message box.

### Interface

- The installed Squadron reopens at the size it had, and maximized if it was.

## 0.34.0 · 2026-11-05

### Conversation

- After an interruption, ↑ in the empty composer takes the last message back: it leaves the conversation and Claude's session, which resumes just before it, and returns to the composer with its files and comments.
- Searching a conversation for a common letter no longer freezes Squadron: past a thousand matches, only the current one is highlighted.
- A background task that only sleeps no longer offers View output, which opened an empty terminal.

### Panels

- In the Pull requests panel, Refresh has a circular arrow icon that turns while refreshing, and is the same height as the placement menu and close buttons beside it.


## 0.33.0 · 2026-11-04

### Panels

- **Beside the conversation**: a panel can open beside the conversation, which narrows to make room, instead of over it. The placement buttons in a panel's header become a menu of four placements drawn as thumbnails: over, beside, under and full screen.
- Settings › Panels shows every panel as a row of placements, with a Small, Medium or Large size (a third, half or two thirds of the room). A panel resized by its edge shows Custom.
- The Jira ticket panel is a panel like the others: it resizes by its edge, changes placement from its header and has its own row in Settings › Panels.

### Conversation

- In a question card with several questions, clicking the picked choice again deselects it, so the question can be answered with free text only.
- While an html block is being written, its placeholder shows the Squadron triangle with a copper stroke running around its outline, above “Creating the preview…”.
- In the new agent window, Cancel and Start agent show their shortcut, Esc and Ctrl ↵, inside the button.

## 0.32.0 · 2026-11-04

### Conversation

- **Jira tickets in sent messages**: a ticket attached to a message shows as a chip with its key and title. A click opens the ticket panel beside the conversation, with its description, comments and links.
- The ticket panel opens below the agent header instead of under it.

### Code hosting

- Settings › Code hosting shows Bitbucket, GitLab, Azure DevOps and GitHub as cards with their logo and account. A host's connection steps open under its row when you click Connect.

### Pull requests

- Ask an agent and Review new changes show a spinner on the row's button while the agent launches, instead of seeming to do nothing.

### Fixes

- Tokens, including the GitHub one used by gh, are read from the system keyring once per launch instead of at every pull request refresh: a keyring restarted locked no longer brings back the unlock prompt all day.
- Model and effort in the new agent window and on a card read as plain labels again, with the advisor on one line.

## 0.31.0 · 2026-11-04

### Code hosting

- **GitLab and Azure DevOps**: Settings › Code hosting connects to GitLab (gitlab.com or a self-managed instance) and to Azure DevOps with a token kept in the system keyring. The pull request of an agent's branch, the lists of pull requests to review and of your own, links in conversations, comments and replies, the review with its line comments, approval, merge and pipelines work there as on Bitbucket and GitHub.
- Request changes is not available on GitLab yet: the comments are posted and Squadron says to ask for changes on GitLab.

## 0.30.0 · 2026-11-04

### Conversation

- **Find in conversation**: Ctrl+F opens a search bar in the conversation, with the count of matches and previous and next.
- **MCP connectors**: a chip in the conversation bar lists the MCP connectors the agent has, with a filter. It shows once the agent has at least one MCP server and can be hidden from the layout settings.

### Pull requests

- **Request changes**: a pull request can be sent back from its menu, next to Approve.

### New agent

- Model and effort change right from the summary of the new agent window and of a Kanban card, without opening the settings. The settings stay closed each time the window opens.

## 0.29.1 · 2026-11-04

### Security

- The built app ships a Content Security Policy: scripts load only from the bundle, so an injected string can no longer run or fetch code.
- The MCP token is no longer visible to every process on the machine: Claude Code reads it from a private file.
- A remembered sudo, ssh or git password is tied to the command it answered, and the card shows that command, so another command started by an agent can no longer replay it silently.
- A dangling symlink in a project can no longer make the editor or ask_values write outside of it; a task text starting with a dash can no longer be read as a Claude Code option.

### Fixes

- Squadron starts again from its AppImage on distributions other than Ubuntu, and a link or file that GIO cannot open, as on Fedora, opens through xdg-open.
- A file dropped on the new agent window joins its attachments instead of the conversation behind it. A file copied in the file manager and pasted is attached instead of pasted as its path, in the conversation too.
- A GitHub pull request where you were just asked for a review shows in the review list right away, without waiting for GitHub's search index.
- An agent waiting for a background command no longer gets stuck forever.
- A Bitbucket project whose remote points to ssh.bitbucket.org is still recognized.
- ask_values tells the agent that the values were written into the file without being returned to it.

## 0.29.0 · 2026-11-03

### Startup

- **Opening screen**: while Squadron starts, a squadron forms in the middle of the window. Wingmen join the formation as the agents come back, under the step being done and the count of agents; then it greets you by your first name and flies off to reveal the app. A click or a key skips it.
- Squadron is ready in half a second even with many long conversations: an agent's history is read only when its conversation opens or a search needs it, and the window no longer freezes while the agents come back.

## 0.28.1 · 2026-11-03

### Fixes

- The remote control chip of a conversation turns remote control on at a click, without a menu; the menu stays for when it is on. The chip no longer grows when turned on.
- Every way of opening a panel slides it in like its button does: the Pull requests button of the title bar, the Overview, the command palette, a pull request card, a task's Kanban button. Opening the list of pull requests closes the panel open in the conversation, so it no longer comes back when the list closes.
- The menus of the title bar and of the conversation bar, such as Pull requests to review or Actions and pipelines, open above the Kanban or Pull requests panel instead of under it.

## 0.28.0 · 2026-11-03

### Agents

- **Task list in the header**: an agent's task list moves from the bottom of the conversation to a chip in its header that opens it. A list whose tasks are all done gives way to a new one at the next message or task, and the previous list stays at hand in the menu.

### Pull requests

- Review on a pull request card or on the reviewing chip of a conversation opens only that pull request's Diff, without the list of pull requests.
- The review panel slides away when closed, like the other panels.

## 0.27.0 · 2026-11-03

### Questions

- **Your own answer per question**: in a card with several questions, each question has its own field, for a free answer or a comment on the chosen option, sent along with it. A × clears it. The frame no longer jumps to the next unanswered question.

### Agents

- On reopening Squadron, each project shows the conversation last visited in it instead of its first one.
- An agent whose goal was sent while Squadron was closed no longer stays in progress forever after its turns end.
- The panel under an agent's header no longer flashes white when switching from the Overview to a project.

### Pull requests

- The Diff of a pull request opened from the Pull requests panel covers the panel from its top, header included.

## 0.26.0 · 2026-11-02

### Panels

- **Placement per panel**: Terminal, Editor, Diff, Subagents, Artifacts, Kanban and Pull requests each open over the conversation, full screen, or under the conversation, which then shrinks. The buttons in the panel’s header and Settings › Panels set it panel by panel, with its own width or height.
- **Transitions**: a panel opening the same way and at least as big slides over the open one; otherwise the open one folds away while the new one opens. Clicking an agent in the list closes the open panel and shows its conversation.

### Pull requests

- **Finish review**: on a pull request you review, it asks for the verdict, Comment, Approve or Request changes, and submits it on GitHub or Bitbucket. A pull request a review agent already reviewed shows that agent; one waiting for your review offers Ask an agent in its Review menu. A review agent’s comments land only on lines the diff shows, and GitHub refusals say why.
- **Your comments** show your avatar ringed in copper. A comment on a removed line with a long line number is no longer cut in the file tree, the review bar wraps on a narrow panel, and on Bitbucket a failed approval no longer posts the summary twice on retry.

### Agents

- **Context gauge**: the new tokens fall into the ring one by one and the level rises with each.
- **Pinned agents** keep the order you gave them whatever the sort. While sorted, only pinned agents can be dragged, and dragging no longer turns the sort off.
- A task card in a conversation has Go to the agent and View the task.
- An agent resuming after its background tasks restarts the turn timer and token counts.
- In Settings, Save stays greyed until something changes.

## 0.25.0 · 2026-11-02

### Pull requests

- **Comments in the review**: each comment shows under its file in the file tree, with its line, its author's avatar and name and the avatars of those who replied. A click lands on the line with its thread highlighted. ‹ 2 of 4 › in the review bar, N and Shift N go from one comment to the next across the pull request. Your own comments show under their line instead of an icon on it, and each message of a thread shows its author's avatar.
- **Review new changes**: when the author pushes after your review, Review new changes sends the new commits to the review agent opened from Pull requests, in its own conversation; See review agent leads to it. A pull request you approved stays approved when new commits come and no longer waits for you. Review agents show their pull request number and the verdict posted in the agent list.
- **Merge or review**: an agent's pull request shows Merge only when you wrote it, and Approve or Request changes otherwise. A pull request opened from an agent's diff shows its review bar again, even before any comment.

### Agents

- **Advisor**: each agent can run with an advisor, Sonnet, Opus or Fable, that Claude consults at key moments. The New agent window shows the message with the model, effort and advisor under it; Settings unfolds the name, Done when, permissions and agent. The conversation shows each consultation in lilac, then its advice, with its tokens. The agent header and menu change the advisor, and /advisor typed in an agent changes only that agent.
- **Agent header**: one button shows the model, its effort as bars in the level's color and the advisor; the permission mode is a badge in its Claude Code color. The context is a ring filling green, orange then red, showing the percentage, the tokens or both.
- **Background tasks**: an agent whose turn ended while a command it started in the background still runs stays working. Its row says what it waits on, the Kanban card shows it, and the conversation shows an In the background card with each task's elapsed time, a View output link and a Stop button.

### Conversation

- **File paths**: a file path written as inline code in a reply, alone or with its :line, opens the file in the editor on click when it exists in the project.

## 0.24.0 · 2026-10-30

### Editor

- **Find in the file**: Ctrl F opens a bubble over the code, top right, with case, whole word and regexp toggles, the match count and a Replace row behind the arrow.
- **Search**: Ctrl Shift F or the Search button opens a palette. Type part of a name to open a file, add :42 to land on line 42. Tab switches to Text, which searches all the project's files, shows the lines around the selected result and opens the file at that line with the search filled in.

## 0.23.1 · 2026-10-30

### Fixes

- **Live thoughts**: while Claude works, each thought of the turn shows whole as its own block between the steps, the current one marked by the accent and a pulse, instead of a scrolling window. Once the answer comes, they fold back into the steps.

## 0.23.0 · 2026-10-30

### Conversation

- **Live thoughts**: while Claude works, the thoughts of the turn show under the steps in a scrolling window that follows the latest one, each separated by its duration and the actions in between. Scrolling up brings a Follow button, and the window fades at the edges where text goes on. When the turn ends, the thoughts fold back into the steps.

## 0.22.0 · 2026-10-30

### Interface

- **Panels over the conversation**: Terminal, Editor, Diff, Kanban and Pull requests slide in from the right over the conversation, under the agent's toolbar. They share one width; full screen stops at the agents list; switching tools swaps the panel and closing slides it out.
- **Agents list**: it no longer folds into avatars and the Rail setting goes. Ctrl B hides the list; the pin in its header brings it back.
- **Pull requests**: the title bar button says Pull requests next to its count.

### Conversation

- **File cards**: a link to a project file written as an absolute path also becomes a file card.

### Fixes

- **Hidden agents bar**: it opens as soon as the mouse moves over the left edge, even when the mouse was already there.

## 0.21.0 · 2026-10-30

### Interface

- **Conversation width**: Settings › Preferences sets it to narrow, normal, wide, all the room, or a custom width in pixels.
- **Agents bar**: always shown, hidden until the mouse reaches the left edge, or a rail of avatars that opens the list on hover. The pin in its header switches between shown and hidden.
- **⋯ menu next to Send**: change the model, the effort and the permissions in submenus, each with its description.

### Conversation

- **Project files**: a link to a project file in the agent's reply becomes a card: open it in the Editor panel, unfold its content in the conversation, or open it in a window of its own.
- **Message menu**: right-clicking a Claude reply offers Copy; the menu shows icons.

### Tickets

- **Jira**: a Jira button opens the sprint tickets in their status columns, a tab per Jira project, with quick filters by person, status, type, priority and epic. The ticket panel shows its status, assignee and priority.

### Artifacts

- **Window of its own**: the Artifacts panel opens in a separate window; pointing at an element there quotes it in the agent's message.

### Pull requests

- **Review from the agent**: Approve and Request changes also work from an agent's view.

### Fixes

- **Permissions**: the file and Kanban windows no longer deny an agent's permission requests while open.

## 0.20.0 · 2026-10-30

### Claude Code

- **/config and /mcp**: in the conversation, /config and /mcp answer as a card: the settings of Claude Code to search and set in a click, and the MCP servers with their status to authorize, reconnect, enable or disable. Other command outputs keep their line breaks.

### Fixes

- **Pull request review**: Approve posts the review right away, no need to open Finish review first.

## 0.19.1 · 2026-10-30

### Fixes

- **Release build**: the build of the release runs on Node 24, with no deprecation warning.

## 0.19.0 · 2026-10-30

### Claude Code

- **/config**: in the conversation, /config lists the settings of Claude Code with their description, then the values of the one chosen. All the settings opens Settings › Claude Code.
- **/mcp**: /mcp lists the MCP servers of the agent with their status. Choosing a server that needs authorization opens it in the browser, choosing a failed one reconnects it.
- **Settings › Claude Code**: every setting of Claude Code, to search and apply, with those of the terminal alone set apart.

### Windows

- **Windows**: Squadron is built for Windows too, as an installer that updates itself. Claude Code must be its native install (claude.exe).

## 0.18.0 · 2026-10-29

### Projects

- **Groups**: projects can be put in groups and sub-groups. A Projects panel (Ctrl P) floats over the app and lists them all in a tree, with drag and drop and Ctrl-click selection.
- **Tab bar**: it keeps the pinned projects and those whose agents were active within a delay set in the settings. A header button gathers the pull requests to review.
- **Project settings**: a list of projects with a sheet per project (General, Agents, CI, Reviews, Tickets). Project settings in the menu of a tab or of the Projects panel opens the sheet of that project.
- **Defaults per project**: a project can set the model, effort and permission mode its new agents start with. New agents, Kanban tasks, the pilot, imports and pull request reviews use them and fall back to the global defaults.

### Search

- **Actions**: a verb and a project, like « pr uh » or « new nova », open its pull requests or the new agent form. A verb alone lists every project and Tab completes it. Kanban, Diff, Terminal, Editor, Archive, Fork, Interrupt, Pin, Overview, each Settings tab, Customize layout and Check for updates are there too. The search of agents and conversations is unchanged.

### Fixes

- **Blurry text**: text is no longer blurry on Linux.
- **Linux PATH**: Squadron reads the login shell’s PATH too, so the claude of ~/.local/bin runs and not an older one.
- **@ mentions**: in a folder that is not a Git repository, @ lists the files without the hidden, dependency and build folders, up to 20,000 paths.
- **Bash rows**: a Bash row always unfolds and shows the whole command, and hovering a tool row shows its whole argument.

## 0.17.0 · 2026-10-29

### Agent mode

- **Start as an agent**: a new session can run as one of Claude Code's agents (project, user or plugin), picked in the new agent form. Its model and effort are prefilled and stay yours to change. The agent stays for the whole session.
- **Agent chip**: the agent shows next to the session title and in the agent list. A click opens its definition, read-only: description, model, effort, skills, the tools it gets sorted by what they let it do, and its prompt with a table of contents.
- **Pilot**: start_agent takes an agent, and overview tells which agent each session runs as.

## 0.16.3 · 2026-10-28

### Fixes

- **Task list**: an agent's task list stays open while some of its tasks are left and folds once they are all done, instead of opening and folding with each turn, which moved the conversation.
- **Mock-ups**: an html block opens straight on its preview once written, instead of showing its HTML for a moment.
- **Messages from another session**: they show offset with an accent line and a « From another session » label.

## 0.16.2 · 2026-10-28

### Fixes

- **Notices from Squadron**: the notice an agent gets when you merge its pull request or post a review of it shows as a discreet card from Squadron, no longer as if you wrote it.

## 0.16.1 · 2026-10-28

### Kanban

- **Tasks in replies**: an agent names a task with a link to its card, which shows as a chip in a sentence or as a card at the head of a list item, for every task, with or without a code. A task whose title starts with a code (T52) is also found when the agent writes the code alone. A chip shows the color of the task's column and opens its window.

## 0.16.0 · 2026-10-28

### Artifacts

- **Artifacts as files**: each agent writes its artifacts in a folder of its own, served apart from the app. Pages run their scripts, reload on each save and let you point at an element to quote it; they go away with their agent.
- **Cleanup**: the artifacts of an agent archived for a month are deleted.

### Kanban

- **Tasks in replies**: a Kanban task an agent names at the head of a list item shows as a card with its column, agent, pull request and Jira ticket. A click opens a window with its goal, description, attachments and links to the Kanban, the agent and the pull request.

### Fixes

- **Messages between agents**: a message sent by another agent shows as a card on the left with its sender, no longer as if you wrote it.
- **Pull request of an agent**: found from the branch it pushed or the link it gave.

## 0.15.0 · 2026-10-28

### Tickets

- **Jira**: Jira connections in Settings › Tickets, Jira projects linked to each project with suggestions from git, a Tickets column in the Kanban with the current sprint, a read-only ticket panel with threaded comments, and a task made from a ticket whose instructions Haiku prepares.
- **Ticket in the header**: an agent working on a task made from a Jira ticket shows the ticket in its header, where it opens to be read again.

### Pull requests

- **Back to the reviewing agent**: a pull request an agent has reviewed offers to go to that agent from its menu, to resume the conversation.
- **Review handed to an agent**: a review in progress goes to the agent working on the branch, or to a new agent in a checkout of the pull request, instead of only being posted on Bitbucket or GitHub.
- **Comments read by agents**: agents read the comments of a pull request, in threads with their replies, with read_pull_comments.

### Agents

- **Removed worktree**: an agent whose worktree was removed opens in its project folder at the next start, instead of showing a stopped session.
- **Predicted reply**: it shows without an arrow after it.

## 0.14.0 · 2026-10-28

### Agents

- **Plan to auto mode**: a plan can be approved straight into auto mode. Once its plan is approved, the agent no longer shows the plan mode.
- **Predicted reply**: after each turn, the empty message field shows the reply Claude Code expects from you. The right arrow takes it.
- **Messages between agents**: a message from another agent shows as a card on the left, with its Markdown rendered and its sender on top.

### Pull requests

- Posting a review, approving or merging a pull request tells the running agents working on it.

### Artifacts

- The panel lists the Claude Docs an agent creates on claude.ai. A Markdown artifact can be published there to share it, with its link to open or copy.

### Fixes

- Agents cut short by an update restart resume again at the next start.

## 0.13.1 · 2026-10-28

### Agents

- **Values and secrets**: the agent no longer edits the file to prepare it. Squadron writes each value where its placeholder is, or adds the line at the end of the file, and creates the file when it does not exist. Auto mode no longer blocks the request.

## 0.13.0 · 2026-10-28

### Agents

- **Picking up after a restart**: an agent cut short by Squadron closing or updating picks up where it left off at the next start, with the messages it had not read yet. It does not restart when it was waiting for your answer.
- **Idle agents**: the Claude Code process of an agent idle for an hour stops, and starts again with its conversation at the next message. Background work, a /loop or Remote Control keeps it running.
- **Artifacts**: an agent can publish a report, a plan or a mock-up in a panel beside its conversation, where it can be copied, saved as a file or deleted.
- **Values and secrets**: an agent can ask for keys or secrets in a card of its conversation, and Squadron writes them into the file without the agent seeing them.
- A stopped session offers Continue, which asks the agent to pick up where it stopped.
- The pilot can pin or unpin an agent with set_pinned, and overview tells which agents are pinned.

### Pull requests

- The agent reviewing a pull request shows which one it reviews, and its instructions can be set for every project, overridden per project, or through the pilot.

### Actions and pipelines

- Each action or pipeline shows the face of the person who started it, with its state in a small badge on the avatar.
- Changing a project's repository looks its actions and pipelines up again right away.
- A screenshot you attached opens in Squadron's viewer too.

## 0.12.0 · 2026-10-27

### Actions and pipelines

- **In the agent's tools**: actions and pipelines move to a button at the right of the agent's tools, which opens their list. The marker on the project tab is gone.
- **Notification when one ends**: Squadron can send a system notification when an action or a pipeline ends, turned on in Settings › Code hosting and off per project in Settings › Projects.

## 0.11.0 · 2026-10-27

### Conversation

- **Commands from the conversation**: a command typed after ! runs in the agent's folder and shows in a card framed green or red, and Claude sees its output. A bash block in a reply has a Run button that does the same.
- **Passwords**: when sudo, ssh or git asks for a password in an agent's command, Squadron asks for it in a window that shows the agent and the command, and Claude never sees it.
- **Files with @**: typing @ in a message or in a new agent's first message suggests the project's files and folders.
- **Conversations without a folder**, kept in a Conversations project of their own, reached from Add a project and from the empty screen.
- A screenshot opens in Squadron's own viewer, which steps only through that conversation's screenshots.
- Claude puts each terminal command in its own block, and an html block shows a loading state while it is written.

### Projects

- **Actions and pipelines**: the GitHub Actions and Bitbucket Pipelines of a project show above its agents while they run, on one line with a panel on hover, and a finished one stays in view, green or red, for a time you choose.
- Screenshots and files can be pasted or attached when creating a task or a new agent.
- An agent started from a task gets the task title at the top of its prompt.
- An agent that finished and has not been read yet stands out in the list with a green edge.
- The agent list no longer ends with the project's folder and Git branch.

### Squadron

- **What's new** in Settings lists every release's features and fixes in the interface language, and a notice shows it once after an update.

## 0.10.0 · 2026-10-27

### Conversation

- **Files as cards**: files you attach sit above your message with their type and size, and a file Claude hands you shows as the same card in its reply.
- **Visual proposals**: a small html block in a reply is drawn in the conversation with Squadron's fonts and colors, and no script runs in it.
- The message Claude writes before asking a question stays shown above it, no longer folded with the steps.
- A pull request named inside a sentence gets its full card under the paragraph, and the sentence keeps a short #number chip.
- A file dropped on a conversation is only attached: Add a project opens for a folder dropped outside a conversation.

### Pull requests

- **Viewed files** in a review: a ticked file folds, the file list counts the viewed ones, and a file that changes afterwards comes back as Changed.
- The review offers Merge on your own pull request and Approve on the others'; Merge is no longer offered on pull requests you only review.
- Review sits before Merge on your pull requests.

### Squadron

- Questions and notices, such as updating, merging or deleting, show in a Squadron dialog.
- Switching project goes back to its conversations.
- Project tabs have a clearer outline, and a tab that rings for you a full one with a glow.
- A relaunched subagent gets its own progress and end.

## 0.9.0 · 2026-10-27

### Pull requests

- **Pull request cards**: a GitHub or Bitbucket pull request linked in an agent reply shows as a card with its live state, checks, approvals, comments and size, and Review, Open with Squadron and Open on GitHub or Bitbucket. A link inside a sentence becomes a pill that shows the card on hover.
- **Review your own pull requests**, from Review or under the Merge arrow; the review comments only, since hosts refuse approving your own.
- **Branch view** in the agent's Diff panel: everything the branch changes since it left the main branch, commits and uncommitted work together, to review and comment before a pull request.
- Opening a comment in a pull request scrolls only the diff, no longer the whole window.

### Screenshots

- Screenshots tools return while an agent works show as thumbnails at the end of the steps line; past four, +N unfolds them all, and a click opens them full size.
- Agents can show a screenshot in their reply from Squadron's captures folder. Captures are deleted after a week.

### Conversation

- What came before the last compaction is folded behind one line, with Show to unfold it.
- Answered questions stay unfolded; the answers of a question stack one under the other.
- The editor shows Markdown and HTML files formatted, with a Preview button.
- A message from another Claude session shows only its text.

### Tabs and links

- A project tab loses its red mark, and its waiting count, once you have looked at its failed agents.
- Web links open in a new tab of the running browser instead of a new window.
- An agent that goes into a worktree with cd shows that worktree's branch.

## 0.8.0 · 2026-10-26

### Context and cost

- **Context gauge** in the agent header, as in Claude Code: 65k / 1M · 7%, where auto-compaction starts, and a Compact now button.
- **Compaction size** in Settings › Preferences: compact around 600k, 400k or 200k instead of Claude Code's 967k.
- **Cold conversation guard**: writing to a large conversation idle for over an hour offers to compact it first, since its cache has expired.

### Agents

- **Select several agents** with Ctrl and a sweep, then archive, pin, mark read or unread, or delete them together.
- **Search everything** with Ctrl+Space: agents, projects and conversations, archived ones included.
- **Model badge**: Opus 5.5 · Extra, with efforts named as in the Claude app.
- Slash command echoes such as the output of /compact no longer show as your messages.
- A session that only went into a folder with cd no longer passes for a worktree.

### Pull requests

- The page spans the full width, with one button per row and the other actions under its arrow.
- The pull request line of an agent card lines up with the others.
- Question cards render cleanly.

## 0.7.0 · 2026-10-26

### Squadron in French

- **The interface speaks your language**: it follows the system language, or the one chosen in Settings › Preferences. French is the first translation, on the whole interface.

### Agents

- **Unread agents**: an agent that finishes while you look elsewhere gets a check badge on its project tab and a dot on its card until you open it. Right-click a card to mark it read or unread.
- **A ringing tab**: a project tab shakes while one of its agents waits for you, and can chime too (Settings › Preferences).
- **Background subagents** are listed and followed, and their agent stays working until they end.
- **Progress and panel**: an agent can show a progress bar on its card, and fill a panel under its conversation header with its own layout, rendered in a sandboxed frame you can hide.
- **Pinned agents** sit above a divider in the list.
- **Costs are gone** from the agent list, the conversation and the overview.
- **Back and forward** between views with the mouse buttons or Alt+← and Alt+→.

### Pull requests

- **Approve and merge** a pull request from Squadron, when its host allows it.
- **Unfold the diff** around each hunk, or show the whole file.
- **Ask an agent** to review keeps you on the pull requests: a banner tells it is reviewing and opens its conversation on demand.
- The conversation rail shows a mark per message, grouped when they no longer fit.

## 0.6.0 · 2026-10-26

### Projects and tabs

- **Choose the color when adding a project**, with the same swatches as its settings.
- **Order the project tabs** in Settings › Projects: by hand, by name or by last activity. Pinned projects stay first. Drag a tab in the header to move it; dragging while sorted by name or activity switches to the order by hand.
- **A project with no agent** opens the Editor, the Terminal, the Diff, the Pull requests and the Kanban.

### Agents

- **Drag to reorder**: agents in the list (sorting “By hand”), the tool buttons and the chips of an agent, without the customize mode.
- **Long conversations open fast**: only the last 20 messages render, earlier ones load as you scroll up. The rail groups its marks when they no longer fit.
- **Thinking in the conversation**: Settings › Preferences › Thinking › “In the conversation” shows Claude's thoughts as a quieter message between its actions.

### Fixes

- The snake and pulse animations run in step everywhere, the header tabs included.
- The app is named “Squadron” in the applications menu and on macOS, and the Squadron menu says macOS on a Mac.

## 0.5.2 · 2026-10-23

### Fixes

- Conversations that start with pasted text now show in Import conversations, and keep those messages once imported.

## 0.5.1 · 2026-10-23

### Fixes

- The macOS build: the settings dialog no longer clashes with the settings module on a case-insensitive disk.

## 0.5.0 · 2026-10-23

### macOS

- **Squadron runs on macOS**, Apple Silicon and Intel. Install it with the same one-line script, which puts it in `/Applications`, or download the `.dmg`. It is not notarized by Apple: if macOS refuses to open the `.dmg` version, click **Open Anyway** once in System Settings › Privacy & Security.
- On macOS, the Bitbucket token goes to the Keychain, the status bar shows the CPU and memory, the editors in /Applications show up in Open with, and conversations of the Claude app can be imported.
- Updates reach Macs the same way: **Update** in the status bar.

### Fixes

- A `cd` into a subfolder no longer passes for a worktree: Squadron switches only to a folder that is a git checkout of its own.

## 0.4.0 · 2026-10-23

### Pull requests

- **Review page by page, file after file**: a pull request review shows every file one after the other, each with its conversations on their lines. The file tree follows the scroll and jumps to a file; each file's header stays on top and folds.
- **New changes since your review**: a pull request you reviewed comes back to the top when its branch moves, and its review opens on what changed since, with All changes one click away.
- **Statuses by who has to act**: solid when it is your turn (review, review again, fix, merge), muted when you wait for others. Each pull request shows its author's avatar.
- A review left as a comment counts as reviewed.
- Draft reviews survive a restart of Squadron.

### Agents

- **Agents can message other agents**: with the new `message_agent` pilot tool, an agent asks to send an instruction to another agent; you approve each message.
- Each agent keeps its unsent message, attachments and diff comments when you switch views, and across restarts.

### Fixes

- The sort menu no longer spills out of the window.
- The collapse button of the agent list sits next to the project name.

## 0.3.0 · 2026-10-23

### Import conversations

- **Bring in conversations held elsewhere**: the button at the far right of the agent list header lists the conversations of the project and its worktrees, from the Claude app or Claude Code, with their title, date and size. Filter them, select them and import them.
- Each one becomes an agent that continues the conversation in a fork: the original stays untouched and keeps working in the Claude app.
- Agents can do it too, with the new `import_session` pilot tool.

### Updates

- **Check for updates** from the Squadron menu or Settings › About. The menu now shows the real version.

### Interface

- The agent list header puts the project name on its own line, with the agent count and the buttons below.

## 0.2.0 · 2026-10-23

### Pull requests

- **Pull request of each agent**: a chip next to the branch shows its state, each reviewer with their avatar and verdict, how many approved, the builds and the comments. Bitbucket works with an API token kept in the system keyring (Settings › Code hosting), GitHub with the gh CLI.
- **Review in the Diff panel**: the pull request diff computed from your checkout, reviewers' threads on their lines, replies, and your comments on lines or on the whole pull request. Gather them in a review, then send it to the agent or post it with Comment, Approve or Request changes.
- **Pull requests page** for each project: the ones waiting for your review and yours, with their progress. A badge on the project tab and a button in the Overview show when one needs you.
- **Ask an agent**: an agent reviews a pull request in its own worktree and drops its remarks into your draft review. Nothing is posted without you.

### Conversations

- **Screenshots and files**: paste them with Ctrl+V, drop them or attach them. Claude sees the images and reads the other files where they are. Pasted screenshots go away with their agent.
- **New agent**: every field is optional. Write the first message in the conversation, with screenshots and files.
- **Code blocks**: syntax colours for every language and a Copy button. Markdown blocks show rendered, with their source one click away. Claude puts text to copy in a block instead of a file.
- Claude Code notices show as a discreet block, and working agents show a readable step instead of raw commands.

### Diff

- Changed files as a folder tree, with a filter.

## 0.1.2 · 2026-10-23

Adds a .deb package: it uses the system's WebKit, which fixes the grey window on recent Ubuntu.

## 0.1.1 · 2026-10-23

Builds on Ubuntu 24.04: fixes the blank window on recent systems.

## 0.1.0 · 2026-10-23

First release.
