# fruit 🍎

A sharing economy simulation game, loosely inspired by [HeyTaco](https://heytaco.com/).

## Concept

Players each hold a bag of fruit. At each game tick all members of a community receive
new fruits at random. The probability of receiving rarer fruits is influenced by two luck
scores:

- **Personal luck** — affects fruit rarity for the individual.
- **Community luck** — affects fruit rarity for all members.

Both scores are normalised floats in `[0.0, 1.0]` (stored internally as `u16`).

### Luck adjustments

Luck is recalculated at each **grant** based on what happened since the previous one.
Actions that accumulate between grants:

| Action | Effect |
|--------|--------|
| Gifting fruit to another player | +personal luck (gifter), proportional to rarity and value |
| Burning fruit | +community luck, proportional to rarity and value |
| Ostentatious gift — gift value greatly exceeds recipient's bag value | −personal luck (gifter) |
| Ostentatious burn — burn value greatly exceeds community average bag value | −personal luck (burner) |
| Quid-pro-quo gifting — reciprocal gifts of similar but unequal value between the same two players | −community luck |

The game rewards generosity and communal contribution; it penalises status-signalling
and transactional behaviour.

**Ostentation** is judged relative to the recipient's or community's bag value *at the
time* the gift or burn was recorded. A large gift to a rich player is less ostentatious
than the same gift to a player holding nothing.

**Quid-pro-quo** is detected from the 100 most recent gift events: the more frequently
pairs of members exchange gifts of similar (but not exactly equal) value in both
directions, the larger the community luck penalty.

## Fruits

Fruits are divided into three **categories**. At neutral luck the approximate per-draw
probabilities are:

| Category | Count | Drop range (neutral luck) |
|----------|-------|--------------------------|
| Standard | 9 | ~1/15 (most common) – ~1/22 (rarest) |
| Rare | 9 | ~1/19 – ~1/48 |
| Exotic | 8 | ~1/96 – ~1/958 |

Higher luck suppresses standard drops and boosts rare and exotic ones. At maximum luck
the exotic range compresses to roughly 1/21 – 1/104 and the rarest exotics benefit most.

Within each category a **rarity** score normalised to `[0.0, 1.0]` determines relative
drop weight; higher means rarer within the tier.

### Standard

| Emoji | Name |
|-------|------|
| 🍇 | Grapes |
| 🍈 | Melon |
| 🍉 | Watermelon |
| 🍊 | Tangerine |
| 🍋 | Lemon |
| 🍌 | Banana |
| 🍍 | Pineapple |
| 🍎 | Red Apple |
| 🍏 | Green Apple |

### Rare

| Emoji | Name |
|-------|------|
| 🍐 | Pear |
| 🍑 | Peach |
| 🍒 | Cherries |
| 🍓 | Strawberry |
| 🥑 | Avocado |
| 🥒 | Cucumber |
| 🥜 | Peanut |
| 🥝 | Kiwi |
| 🥥 | Coconut |

### Exotic

| Emoji | Name |
|-------|------|
| 🥭 | Mango |
| 🍅 | Tomato |
| 🌰 | Chestnut |
| 🌶 | Hot Pepper |
| 🫑 | Bell Pepper |
| 🫚 | Ginger Root |
| 🫐 | Blueberries |
| 🫒 | Olive |

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
| `make tc` | Test coverage (requires `cargo-llvm-cov`) |
| `make c` | Type-check (`cargo check`) |
| `make b` | Debug build |
| `make r` | Run REPL |
| `make br` | Release build |

### REPL

`make r` launches an interactive REPL for testing the game loop:

```
> add Alice
> add Bob
> luck 0.3                      # set community luck (0.0–1.0)
> luck Alice 0.8                # set member luck
> grant 5                       # grant 5 fruits to every member (applies luck adjustments)
> gift Alice Bob 🍓             # Alice gives Bob one strawberry
> burn Alice 🍇 🍇              # Alice burns two grapes
> log 10                        # show the 10 most recent log records
> remove Bob
> quit
```
