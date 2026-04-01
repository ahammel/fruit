# gib fruit 🍎

A sharing-economy simulation game, loosely inspired by [HeyTaco](https://heytaco.com/).

## Concept

Players each hold a bag of fruit. The game progresses in clock ticks; at each tick, players receive new fruits at random. The rarity of those fruits is influenced by two luck scores:

- **Personal luck** — affects fruit rarity for the individual.
- **Community luck** — affects fruit rarity for all players.

### What raises luck

| Action | Effect |
|--------|--------|
| Gifting fruit to another player | +personal luck |
| Burning fruit | +community luck |

### What lowers luck

| Action | Effect |
|--------|--------|
| Ostentatious gifts the recipient could not plausibly reciprocate | −personal luck |
| Quid-pro-quo trades | −community luck |

The game rewards generosity and communal contribution; it penalises status-signalling and transactional behaviour.

## Fruits

Rarity is a normalised score in `[0.0, 1.0]`; higher means rarer. Fruits in the exotic tier (rarity ≥ 0.80) are uncommon drops that signal meaningful generosity.

| Emoji | Name | Rarity |
|-------|------|--------|
| 🍇 | Grapes | 0.0010 |
| 🍈 | Melon | 0.0016 |
| 🍉 | Watermelon | 0.0021 |
| 🍊 | Tangerine | 0.0026 |
| 🍋 | Lemon | 0.0031 |
| 🍌 | Banana | 0.0036 |
| 🍍 | Pineapple | 0.0042 |
| 🍎 | Red Apple | 0.0047 |
| 🍏 | Green Apple | 0.0052 |
| 🍐 | Pear | 0.0057 |
| 🍑 | Peach | 0.0063 |
| 🍒 | Cherries | 0.0068 |
| 🍓 | Strawberry | 0.0073 |
| 🥑 | Avocado | 0.8020 |
| 🥒 | Cucumber | 0.8023 |
| 🥜 | Peanut | 0.8075 |
| 🥝 | Kiwi | 0.8082 |
| 🥥 | Coconut | 0.8123 |
| 🥭 | Mango | 0.8164 |
| 🍅 | Tomato | 0.8500 |
| 🌰 | Chestnut | 0.8600 |
| 🌶 | Hot Pepper | 0.8750 |
| 🫑 | Bell Pepper | 0.9200 |
| 🫚 | Ginger Root | 0.9700 |
| 🫐 | Blueberries | 0.9990 |
| 🫒 | Olive | 1.0000 |

## Integrations

Designed to run inside chat platforms:

- Slack
- Discord
- Microsoft Teams

## Development

### Prerequisites

- Rust (stable)

### Common commands

| Command | Description |
|---------|-------------|
| `make p` | Format (`rustfmt`) |
| `make l` | Lint (`clippy`) |
| `make t` | Run tests |
| `make c` | Type-check (`cargo check`) |
| `make b` | Debug build |
| `make r` | Run |
| `make br` | Release build |
