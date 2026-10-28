# Changelog

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
