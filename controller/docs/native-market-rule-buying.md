# Native Moth Watch rule buying

Moth Controller's **Automation → Market rule buying · Moth Watch** panel supports native, per-account market buying without Violentmonkey or a browser userscript. It is independent of native Auto Catch, shop restocking and server automation.

## Configuring

1. Select the intended account. Market settings are saved separately for Game1–Game4, including when that account is offline.
2. Add one or more specific-item, any-item or Pokémon rules, set currency (Gold or Gems) and a **maximum unit price**, and enable the individual rules. For Pokémon, select **Any**, **P5** or **Shiny**. Shiny requires Gems.
3. Optionally specify account Gold/Gem reserves and global/per-rule spending, purchase-count or quantity limits. A zero limit means *unlimited*, not zero spend.
4. Enable **native market buying** and click **Save market rules**. The engine is disabled by default. Pausing or disabling only affects this market buyer; other native automation stays active.

The per-account order journal is saved as `market-orders-GameN.json` under Moth's app-data directory. The configuration is `market-rules-GameN.json`. Only rules and transaction exposure are stored; authentication credentials are not copied into these files.

## Importing existing Moth Watch settings

From a Firefox profile where browser Moth Watch was configured, copy the JSON value returned by:

```js
localStorage.getItem('moth-pokeidle-market-bot-config-v1')
```

Paste it into **Import existing Moth Watch rules**. The imported settings preserve available rule types, prices and limits, but leave the global native buyer **OFF** until reviewed and saved. Existing browser scripts are not modified, and their journal is not imported.

**Avoid running the browser Moth Watch buyer and native buyer at the same time for the same account**. Two independent spending engines cannot coordinate transaction exposure.

## Market request and confirmation model

The authenticated native game WebSocket issues `market.itens` summaries (only for broad item rules), `market.item` exact offer queries, and `market.listar` Pokémon pages. Quotes are rejected when stale, held, missing required identity/price data, or offered by the account itself. Only actual item/Pokémon listing replies authorize `market.comprar`, never a summary minimum or historical average. For a qualified item listing, the buyer requests the *maximum quantity available and affordable* in one purchase.

Only one market purchase is pending on a native account at a time so that a server `marketComprado` event without a listing ID can be attributed reliably. A matched receipt confirms the purchase; an explicit matching refusal clears its spend; a missing/ambiguous receipt is marked **unverified**, keeps the quoted expenditure reserved, and is never silently retried for that listing. The native scan respects shared-socket rates and server rate-limit backoff. Status displays pending, confirmed and unverified orders.

## Mission automation status

The game-confirmed server automation keys currently managed by the controller cover potions, revives, auto selling, Pokémon locking and hunt return. A mission/quest write-and-acknowledge command has not been verified. The **Mission automation · observed server fields** section is intentionally **read-only**: it exposes mission-like fields *only if the authenticated server snapshot actually supplies them*. No guessed mission commands are sent. Live protocol evidence is required before enabling mission mutations.
