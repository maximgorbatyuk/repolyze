# Features and know issues

## Features

- [ ] RF-1. 'About' screen with project details, license, and privacy policy
- [ ] RF-2. App should show how many installs were made by the users. Investigate we can get this information from the Homebrew.
- [ ] RF-3. App should have option to have global config file for default values and local one. Folder for global configs is `~/.repolyze/config.json`, local one is `<project_path>/.repolyze/config.json`.
- [ ] RF-4. There should be menu items 'git utilities'. The menu will have following options: 'clean merged branches', 'clean stale branches'.
- [ ] RF-5. 'Clean merged branches' command should retrieve and show all branches that were merged into a branch which user tells before analysis. So, flow is the following: user select the option - user gets aksed for target branch (like dev, sandbox, main, etc) - app shows list of branches that should be removed - app asks for confirmation to remove the branches - if user confirms, app removes the branches | if user denies, app returns to the main menu.
- [ ] RF-6. 'Clean stale branches' command should retrieve and show all branches that are stale (not merged into any branch for long time). Flow is the following: user select the option - app asks for branch and count of inactivity days - app shows list of branches that should be removed - app asks for confirmation to remove the branches - if user confirms, app removes the branches | if user denies, app returns to the main menu.
- [ ] RF-7. First menu item shold be called 'Analyze'. Submenu items: 'all', 'Users contribution' (RF-8), 'Most active days and hours' (RF-9), 'Repository project size'  (RF-10).
- [ ] RF-8. 'Users contribution' should show all users with their contribution statistics. What should be calculated: commits, lines added/deleted, average lines modified per commit, files touched, and active days per contribution. Analysis should include all repositories in the folder that was selected for analysis, or one if there is only one repository. Output should be sorted by total commits count per user, most goes first. Output should be ASCII table with headers: 'Email', 'Commits', 'Lines Modified', 'Lines per commit', 'Files Touched', 'Most active week day'.
- [ ] RF-9. 'Most active days and hours' should show most active days of the week and hours of day. Analysis should include all repositories in the folder that was selected for analysis, or one if there is only one repository. Output should be ASCII table with headers: 'Email', 'Most active week day', 'Average commits per day, in the most active day', 'Average commits per day', 'Average commits per hour, in the most active hour', 'Average commits per hour'.

## Known issues
