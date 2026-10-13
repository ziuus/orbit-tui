# Community Themes

A bunch of extra themes you can drop into Orbit. No rebuild, just copy and go.

```sh
mkdir -p ~/.config/orbit/themes
cp themes/night-owl.toml ~/.config/orbit/themes/
# or symlink it so edits show up next launch
orbit link themes/night-owl.toml
```

Then hit `T` until it shows up, or set `theme = "night-owl"` in your `config.toml`.

## Making your own

It's just nine hex colors. Name the file whatever you want the theme to be called:

```toml
# my-theme.toml
bg = "#1a1b26"         # background
surface = "#292e42"    # panels, borders
accent = "#7aa2f7"     # main highlight
secondary = "#bb9af7"  # second highlight
text = "#c0caf5"       # normal text
dim = "#565f89"        # labels, hints
green = "#9ece6a"      # all good
yellow = "#e0af68"     # warning
red = "#f7768e"        # error / high usage
```

Heads up: if a key's missing or a color's off, Orbit just skips the file
without telling you. So if your theme doesn't show up, that's probably why.
Also don't name it after a built-in (`dracula`, `nord`, etc.) or it'll
quietly replace that one.