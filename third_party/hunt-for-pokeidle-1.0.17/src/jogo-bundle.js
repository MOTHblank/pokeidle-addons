// GERADO por scripts/vendor-jogo.mjs — NÃO EDITAR. Os módulos do jogo embutidos (sem import remoto).
(() => {
  var __defProp = Object.defineProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };

  // tests/.cache/shared/cooldown-golpes.mjs
  var cooldown_golpes_exports = {};
  __export(cooldown_golpes_exports, {
    COOLDOWN_600_MS: () => COOLDOWN_600_MS,
    COOLDOWN_MEGA_MS: () => COOLDOWN_MEGA_MS,
    PODER_MINIMO: () => PODER_MINIMO,
    SPEED_POTENCIAL_TETO_PCT: () => SPEED_POTENCIAL_TETO_PCT,
    aplicarCurvaNoGolpe: () => aplicarCurvaNoGolpe,
    cooldownDoPoder: () => cooldownDoPoder,
    cooldownGolpeComSpeed: () => cooldownGolpeComSpeed,
    descontoDoSpeedPct: () => descontoDoSpeedPct,
    poderNormalizado: () => poderNormalizado,
    potencialDoSpeed: () => potencialDoSpeed
  });
  var PODER_MINIMO = 15;
  var PODER_JOELHO = 100;
  var COOLDOWN_600_MS = 6e4;
  var COOLDOWN_MEGA_MS = 3e4;
  function poderNormalizado(power) {
    const p = Number(power);
    return Number.isFinite(p) && p > PODER_MINIMO ? p : PODER_MINIMO;
  }
  function cooldownDoPoder(power) {
    const p = poderNormalizado(power);
    const seg = p <= PODER_JOELHO ? 10 + (p - PODER_MINIMO) * 20 / (PODER_JOELHO - PODER_MINIMO) : 30 + (p - PODER_JOELHO) * 30 / (600 - PODER_JOELHO);
    return Math.round(seg * 10) * 100;
  }
  function aplicarCurvaNoGolpe(a) {
    if (!a) return false;
    const poder = poderNormalizado(a.power);
    const cd = cooldownDoPoder(poder);
    if (a.power === poder && a.cooldownMs === cd) return false;
    a.power = poder;
    a.cooldownMs = cd;
    return true;
  }
  var SPEED_POTENCIAL_TETO_PCT = 7;
  var SPEED_JOELHO = 120;
  var ivDoSpeed = (iv) => Math.min(32, Math.max(1, Math.round(Number(iv) || 1)));
  function potencialDoSpeed(speedBase) {
    const s = Math.max(0, Number(speedBase) || 0);
    const pct = s <= SPEED_JOELHO ? s / 20 : 6 + (s - SPEED_JOELHO) / 40;
    return Math.min(SPEED_POTENCIAL_TETO_PCT, pct);
  }
  function descontoDoSpeedPct(speedBase, ivSpeed) {
    return potencialDoSpeed(speedBase) * (ivDoSpeed(ivSpeed) / 32);
  }
  function cooldownGolpeComSpeed(cooldownMs, speedBase, ivSpeed) {
    const base = Math.max(0, Number(cooldownMs) || 0);
    if (base <= 0) return base;
    return Math.round(base * (1 - descontoDoSpeedPct(speedBase, ivSpeed) / 100));
  }

  // tests/.cache/shared/golpes-especiais.mjs
  var golpes_especiais_exports = {};
  __export(golpes_especiais_exports, {
    GOLPE_600_POR_TIPO: () => GOLPE_600_POR_TIPO,
    GOLPE_600_POR_TIPO_B: () => GOLPE_600_POR_TIPO_B,
    GOLPE_600_RESERVA: () => GOLPE_600_RESERVA,
    TIPOS_ELEMENTO: () => TIPOS_ELEMENTO,
    dexDe: () => dexDe,
    ehUltimaEvolucao: () => ehUltimaEvolucao,
    elegivelParaGolpeEspecial: () => elegivelParaGolpeEspecial,
    fixarCooldownGolpe600: () => fixarCooldownGolpe600,
    golpesDaFicha: () => golpesDaFicha,
    injetarGolpesEspeciais: () => injetarGolpesEspeciais,
    limparGolpesQuebrados: () => limparGolpesQuebrados,
    temGolpe600DoTipo: () => temGolpe600DoTipo
  });

  // tests/.cache/shared/outland.mjs
  var outland_exports = {};
  __export(outland_exports, {
    OUTLAND_DEX_BASE: () => OUTLAND_DEX_BASE,
    OUTLAND_POKE_LEGADO_MAX: () => OUTLAND_POKE_LEGADO_MAX,
    OUTLAND_POKE_LEGADO_MIN: () => OUTLAND_POKE_LEGADO_MIN,
    OUTLAND_POKE_MAX: () => OUTLAND_POKE_MAX,
    OUTLAND_POKE_MIN: () => OUTLAND_POKE_MIN,
    OUTLAND_POKE_REMAP_OFFSET: () => OUTLAND_POKE_REMAP_OFFSET,
    aplicarRemapOutlandLista: () => aplicarRemapOutlandLista,
    dexChaveOutland: () => dexChaveOutland,
    isEspelhoFantasmaPokeId: () => isEspelhoFantasmaPokeId,
    isOutlandPokeId: () => isOutlandPokeId,
    isOutlandPokeIdLegado: () => isOutlandPokeIdLegado,
    montarOutlandDex: () => montarOutlandDex,
    montarOutlandDexLegado48: () => montarOutlandDexLegado48,
    remapOutlandPokeId: () => remapOutlandPokeId
  });
  var OUTLAND_DEX_BASE = 2001;
  var OUTLAND_POKE_MIN = 2001;
  var OUTLAND_POKE_MAX = 2059;
  var OUTLAND_POKE_LEGADO_MIN = 10501;
  var OUTLAND_POKE_LEGADO_MAX = 10548;
  var OUTLAND_POKE_REMAP_OFFSET = OUTLAND_POKE_LEGADO_MIN - OUTLAND_POKE_MIN;
  function isOutlandPokeId(pokeId) {
    const id = Number(pokeId);
    return id >= OUTLAND_POKE_MIN && id < OUTLAND_POKE_MAX;
  }
  function isOutlandPokeIdLegado(pokeId) {
    const id = Number(pokeId);
    return id >= OUTLAND_POKE_LEGADO_MIN && id < OUTLAND_POKE_LEGADO_MAX;
  }
  function remapOutlandPokeId(pokeId) {
    const id = Number(pokeId);
    if (!Number.isFinite(id)) return pokeId;
    if (isOutlandPokeIdLegado(id)) return id - OUTLAND_POKE_REMAP_OFFSET;
    return id;
  }
  function isEspelhoFantasmaPokeId(pokeId) {
    const id = Number(pokeId);
    return id >= 1e4 && id < OUTLAND_POKE_LEGADO_MIN;
  }
  function montarOutlandDex(especiesMap) {
    const lista = [...especiesMap.values()].filter((e) => isOutlandPokeId(e.pokeId)).sort((a, b) => a.pokeId - b.pokeId);
    const pokeToDex = {};
    const dexToPoke = {};
    for (const e of lista) {
      pokeToDex[e.pokeId] = e.pokeId;
      dexToPoke[e.pokeId] = e.pokeId;
    }
    const total = lista.length;
    return {
      pokeToDex,
      dexToPoke,
      total,
      min: OUTLAND_DEX_BASE,
      max: total ? OUTLAND_DEX_BASE + total - 1 : OUTLAND_DEX_BASE
    };
  }
  function dexChaveOutland(pokeId) {
    return isOutlandPokeId(pokeId) ? pokeId : null;
  }
  function montarOutlandDexLegado48(_speciesMap) {
    const dexToPoke = {};
    for (let i = 0; i < 47; i++) {
      dexToPoke[OUTLAND_DEX_BASE + i] = OUTLAND_POKE_LEGADO_MIN + i;
    }
    return dexToPoke;
  }
  function aplicarRemapOutlandLista(lista) {
    for (const c of lista) {
      if (isOutlandPokeIdLegado(c.pokeId)) c.pokeId = remapOutlandPokeId(c.pokeId);
      if (c.evolvesToId && isOutlandPokeIdLegado(c.evolvesToId)) {
        c.evolvesToId = remapOutlandPokeId(c.evolvesToId);
      }
    }
  }

  // tests/.cache/shared/golpes-especiais.mjs
  var GOLPE_600_POR_TIPO = {
    GRASS: { name: "Petal Bloom", power: 600, type: "GRASS", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    WATER: { name: "Cyclonic Tide", power: 600, type: "WATER", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    FIRE: { name: "Ignition Point", power: 600, type: "FIRE", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    FLYING: { name: "Massive Hurricane", power: 600, type: "FLYING", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    BUG: { name: "Hive Crush", power: 600, type: "BUG", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    POISON: { name: "Toxic Deluge", power: 600, type: "POISON", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    ELECTRIC: { name: "Static Overload", power: 600, type: "ELECTRIC", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    GROUND: { name: "Crossing Fissure", power: 600, type: "GROUND", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    FIGHTING: { name: "Meteor Fists", power: 600, type: "FIGHTING", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    PSYCHIC: { name: "Mental Singularity", power: 600, type: "PSYCHIC", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    ROCK: { name: "Crumbling Rain", power: 600, type: "ROCK", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    GHOST: { name: "Untold Nightmare", power: 600, type: "GHOST", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    ICE: { name: "Ice Time", power: 600, type: "ICE", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    DARK: { name: "Endless Hollow", power: 600, type: "DARK", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    STEEL: { name: "Alloy Breaker", power: 600, type: "STEEL", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    DRAGON: { name: "Eternal Dragon", power: 600, type: "DRAGON", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    FAIRY: { name: "Starlight Charm", power: 600, type: "FAIRY", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    NORMAL: { name: "Ultimate Force", power: 600, type: "NORMAL", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS }
  };
  var GOLPE_600_POR_TIPO_B = {
    NORMAL: { name: "Giga Impact", power: 600, type: "NORMAL", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    FIRE: { name: "Blast Burn", power: 600, type: "FIRE", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    WATER: { name: "Hydro Cannon", power: 600, type: "WATER", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    GRASS: { name: "Frenzy Plant", power: 600, type: "GRASS", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    ELECTRIC: { name: "Bolt Strike", power: 600, type: "ELECTRIC", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    ICE: { name: "Glacial Lance", power: 600, type: "ICE", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    FIGHTING: { name: "Meteor Assault", power: 600, type: "FIGHTING", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    POISON: { name: "Gunk Shot", power: 600, type: "POISON", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    GROUND: { name: "Precipice Blades", power: 600, type: "GROUND", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    FLYING: { name: "Sky Attack", power: 600, type: "FLYING", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    PSYCHIC: { name: "Prismatic Laser", power: 600, type: "PSYCHIC", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    BUG: { name: "Megahorn", power: 600, type: "BUG", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    ROCK: { name: "Rock Wrecker", power: 600, type: "ROCK", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    GHOST: { name: "Astral Barrage", power: 600, type: "GHOST", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    DRAGON: { name: "Roar of Time", power: 600, type: "DRAGON", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    DARK: { name: "Wicked Blow", power: 600, type: "DARK", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    STEEL: { name: "Steel Beam", power: 600, type: "STEEL", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    FAIRY: { name: "Light of Ruin", power: 600, type: "FAIRY", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS }
  };
  var GOLPE_600_RESERVA = {
    NORMAL: { name: "Blood Moon", power: 600, type: "NORMAL", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    FIRE: { name: "Mind Blown", power: 600, type: "FIRE", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    WATER: { name: "Steam Eruption", power: 600, type: "WATER", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    GRASS: { name: "Chloroblast", power: 600, type: "GRASS", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    ELECTRIC: { name: "Plasma Fists", power: 600, type: "ELECTRIC", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    ICE: { name: "Freeze Shock", power: 600, type: "ICE", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    FIGHTING: { name: "Combat Torque", power: 600, type: "FIGHTING", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    POISON: { name: "Noxious Torque", power: 600, type: "POISON", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    GROUND: { name: "Land's Wrath", power: 600, type: "GROUND", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    FLYING: { name: "Bleakwind Storm", power: 600, type: "FLYING", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    PSYCHIC: { name: "Psycho Boost", power: 600, type: "PSYCHIC", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    BUG: { name: "Pollen Puff", power: 600, type: "BUG", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    ROCK: { name: "Diamond Storm", power: 600, type: "ROCK", category: "PHYSICAL", cooldownMs: COOLDOWN_600_MS },
    GHOST: { name: "Moongeist Beam", power: 600, type: "GHOST", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    DRAGON: { name: "Dynamax Cannon", power: 600, type: "DRAGON", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    DARK: { name: "Fiery Wrath", power: 600, type: "DARK", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    STEEL: { name: "Make It Rain", power: 600, type: "STEEL", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS },
    FAIRY: { name: "Fleur Cannon", power: 600, type: "FAIRY", category: "SPECIAL", cooldownMs: COOLDOWN_600_MS }
  };
  var TIPOS_ELEMENTO = Object.keys(GOLPE_600_POR_TIPO);
  function dexDe(id) {
    return id < 1e3 ? id : id % 1e3;
  }
  function ehUltimaEvolucao(c) {
    return !c.evolvesToId;
  }
  function elegivelParaGolpeEspecial(c) {
    if (!ehUltimaEvolucao(c)) return false;
    if (isEspelhoFantasmaPokeId(c.pokeId)) return false;
    if (c.megaDeDex != null) return false;
    if (c.golpesFechados) return false;
    return true;
  }
  function temGolpe600DoTipo(attacks, tipo) {
    return (attacks ?? []).some((a) => a.power >= 600 && a.type === tipo);
  }
  function limparGolpesQuebrados(attacks) {
    return (attacks ?? []).filter((a) => {
      if (!TIPOS_ELEMENTO.includes(a.type)) return false;
      if ((a.power ?? 0) <= 10) return false;
      return true;
    });
  }
  function fixarCooldownGolpe600(creature) {
    let mexeu = false;
    for (const a of creature.attacks ?? []) {
      if (a.cdFixo) continue;
      if ((a.power ?? 0) >= 600 && a.cooldownMs !== COOLDOWN_600_MS) {
        a.cooldownMs = COOLDOWN_600_MS;
        mexeu = true;
      }
    }
    return mexeu;
  }
  function injetarGolpesEspeciais(creature) {
    fixarCooldownGolpe600(creature);
    if (!elegivelParaGolpeEspecial(creature)) return false;
    let attacks = limparGolpesQuebrados(creature.attacks);
    const antes = JSON.stringify(attacks);
    const tipos = [...new Set([creature.type1, creature.type2].filter((t) => t && TIPOS_ELEMENTO.includes(t)))];
    const catAssinatura = (creature.baseAtk ?? 0) >= (creature.baseSpAtk ?? 0) ? "PHYSICAL" : "SPECIAL";
    for (const a of attacks) {
      if (a.power < 600) continue;
      if (!a.categoriaFixa) a.category = catAssinatura;
      a.cooldownMs = COOLDOWN_600_MS;
    }
    for (const tipo of tipos) {
      const tmpl = GOLPE_600_POR_TIPO[tipo];
      if (!tmpl || temGolpe600DoTipo(attacks, tipo)) continue;
      attacks.push({ ...tmpl, category: catAssinatura, learnLevel: 1 });
    }
    const golpes600 = attacks.filter((a) => a.power >= 600 && TIPOS_ELEMENTO.includes(a.type));
    if (golpes600.length === 1) {
      const b = [GOLPE_600_POR_TIPO_B, GOLPE_600_RESERVA].map((tabela) => tabela[golpes600[0].type]).find((g) => g && !attacks.some((a) => a.name === g.name));
      if (b) attacks.push({ ...b, category: catAssinatura, learnLevel: 1 });
    }
    if (JSON.stringify(attacks) === antes) return false;
    creature.attacks = attacks;
    return true;
  }
  function golpesDaFicha(especie) {
    if (!especie) return [];
    const clone = { ...especie, attacks: (especie.attacks ?? []).map((a) => ({ ...a })) };
    injetarGolpesEspeciais(clone);
    return clone.attacks ?? [];
  }

  // tests/.cache/shared/golpes-planilhas.mjs
  var golpes_planilhas_exports = {};
  __export(golpes_planilhas_exports, {
    GOLPES_ARTE_NOVA: () => GOLPES_ARTE_NOVA,
    GOLPES_ARTE_NOVA_2026_10_30: () => GOLPES_ARTE_NOVA_2026_10_30,
    GOLPES_ARTE_NOVA_2026_11_05: () => GOLPES_ARTE_NOVA_2026_11_05,
    GOLPES_CONTRATO_2026_10_16: () => GOLPES_CONTRATO_2026_10_16,
    GOLPES_DUAS_FORMAS_2026_11_05: () => GOLPES_DUAS_FORMAS_2026_11_05,
    GOLPES_INICIAIS: () => GOLPES_INICIAIS,
    GOLPES_LENDARIOS_2026_10_11: () => GOLPES_LENDARIOS_2026_10_11,
    GOLPES_LENDARIOS_2026_10_27: () => GOLPES_LENDARIOS_2026_10_27,
    GOLPES_LENDARIOS_2026_10_29: () => GOLPES_LENDARIOS_2026_10_29,
    GOLPES_MEGAS_2026_10_03: () => GOLPES_MEGAS_2026_10_03,
    GOLPES_MEGAS_2026_10_21: () => GOLPES_MEGAS_2026_10_21,
    GOLPES_MEGAS_2026_10_22: () => GOLPES_MEGAS_2026_10_22,
    GOLPES_MEGAS_2026_10_23: () => GOLPES_MEGAS_2026_10_23,
    GOLPES_MEGAS_2026_10_26: () => GOLPES_MEGAS_2026_10_26,
    GOLPES_MEGAS_2026_10_27: () => GOLPES_MEGAS_2026_10_27,
    GOLPES_MEGAS_2026_10_29: () => GOLPES_MEGAS_2026_10_29,
    GOLPES_MEGAS_2026_10_30: () => GOLPES_MEGAS_2026_10_30,
    GOLPES_MEGAS_CONTRATO_2026_10_16: () => GOLPES_MEGAS_CONTRATO_2026_10_16,
    GOLPES_MENOS_DE_10: () => GOLPES_MENOS_DE_10,
    GOLPES_OUTLAND_2026_10_13: () => GOLPES_OUTLAND_2026_10_13,
    GOLPES_OUTLAND_2026_10_15: () => GOLPES_OUTLAND_2026_10_15,
    GOLPES_OUTLAND_2026_10_27: () => GOLPES_OUTLAND_2026_10_27,
    GOLPES_OUTLAND_2026_11_02: () => GOLPES_OUTLAND_2026_11_02,
    GOLPES_OUTLAND_2026_11_05: () => GOLPES_OUTLAND_2026_11_05,
    GOLPES_PRE_EVOLUCOES_KANTO: () => GOLPES_PRE_EVOLUCOES_KANTO,
    GOLPES_PRIMEIRO_ESTAGIO_2026_11_05: () => GOLPES_PRIMEIRO_ESTAGIO_2026_11_05,
    GOLPES_PSEUDO_LENDARIOS: () => GOLPES_PSEUDO_LENDARIOS,
    GOLPES_REWORK_2026_09: () => GOLPES_REWORK_2026_09,
    GOLPES_REWORK_2026_09_28: () => GOLPES_REWORK_2026_09_28,
    GOLPES_REWORK_2026_10_03: () => GOLPES_REWORK_2026_10_03,
    GOLPES_REWORK_2026_10_04: () => GOLPES_REWORK_2026_10_04,
    GOLPES_REWORK_2026_10_13: () => GOLPES_REWORK_2026_10_13,
    GOLPES_REWORK_2026_10_15: () => GOLPES_REWORK_2026_10_15,
    GOLPES_REWORK_2026_10_23: () => GOLPES_REWORK_2026_10_23,
    GOLPES_REWORK_2026_10_26: () => GOLPES_REWORK_2026_10_26,
    GOLPES_REWORK_2026_10_27: () => GOLPES_REWORK_2026_10_27,
    GOLPES_REWORK_2026_10_29: () => GOLPES_REWORK_2026_10_29,
    GOLPES_REWORK_2026_11_02: () => GOLPES_REWORK_2026_11_02,
    GOLPES_SEGUNDA_EVO_2026_10_15: () => GOLPES_SEGUNDA_EVO_2026_10_15,
    GOLPES_SEM_EVOLUCAO_2026_11_05: () => GOLPES_SEM_EVOLUCAO_2026_11_05,
    aplicarGolpesDasPlanilhas: () => aplicarGolpesDasPlanilhas,
    golpesDaPlanilhaDaMega: () => golpesDaPlanilhaDaMega,
    golpesDaPlanilhaDoOutland: () => golpesDaPlanilhaDoOutland,
    golpesDoOutlandFechados: () => golpesDoOutlandFechados
  });
  var GOLPES_INICIAIS = {};
  var GOLPES_PSEUDO_LENDARIOS = {};
  var GOLPES_MENOS_DE_10 = {
    217: [
      // Ursaring — NORMAL
      ["Bind", "NORMAL", "PHYSICAL", 15],
      ["Fury Attack", "NORMAL", "PHYSICAL", 15],
      ["Fury Swipes", "NORMAL", "PHYSICAL", 18],
      ["Scratch", "NORMAL", "PHYSICAL", 40],
      ["Snore", "NORMAL", "SPECIAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Slash", "NORMAL", "PHYSICAL", 70],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ]
    // O Milotic (#350) saiu daqui em 23/10/2026 para `GOLPES_REWORK_2026_10_23`; hoje mora em `GOLPES_REWORK_2026_11_02`.
  };
  var GOLPES_REWORK_2026_09 = {
    57: [
      // Primeape — FIGHTING
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Power-Up Punch", "FIGHTING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 55],
      ["Upper Hand", "FIGHTING", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Superpower", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ]
  };
  var GOLPES_PRE_EVOLUCOES_KANTO = {};
  var GOLPES_REWORK_2026_09_28 = {};
  var GOLPES_REWORK_2026_10_03 = {};
  var GOLPES_REWORK_2026_10_04 = {
    292: [
      // Shedinja — BUG/GHOST
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Shadow Punch", "GHOST", "PHYSICAL", 55],
      ["Team Slice", "BUG", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["U-turn", "BUG", "PHYSICAL", 70],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Night Shade", "GHOST", "SPECIAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 140],
      ["Sinister Arrow Raid", "GHOST", "PHYSICAL", 180],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ]
  };
  var MEGA = "MEGA";
  var GOLPES_MEGAS_2026_10_03 = {};
  var GOLPES_LENDARIOS_2026_10_11 = {};
  var GOLPES_REWORK_2026_10_13 = {
    625: [
      // Bisharp — DARK/STEEL
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Wicked Blow", "DARK", "PHYSICAL", 600],
      ["Mega Forge", "STEEL", "PHYSICAL", 600]
    ]
  };
  var GOLPES_OUTLAND_2026_10_13 = {
    2020: [
      // Brute Rhydon — GROUND/ROCK · base Rhydon (#112)
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ]
  };
  var GOLPES_REWORK_2026_10_15 = {};
  var GOLPES_SEGUNDA_EVO_2026_10_15 = {
    2: [
      // Ivysaur — GRASS/POISON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Trop Kick", "GRASS", "PHYSICAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    5: [
      // Charmeleon — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    8: [
      // Wartortle — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    11: [
      // Metapod — BUG
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    14: [
      // Kakuna — BUG/POISON
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    17: [
      // Pidgeotto — NORMAL/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    25: [
      // Pikachu — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    30: [
      // Nidorina — POISON
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    33: [
      // Nidorino — POISON
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Noxious Torque", "POISON", "PHYSICAL", 600]
    ],
    35: [
      // Clefairy — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    39: [
      // Jigglypuff — NORMAL/FAIRY
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    42: [
      // Golbat — POISON/FLYING
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Noxious Torque", "POISON", "PHYSICAL", 600]
    ],
    44: [
      // Gloom — GRASS/POISON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    61: [
      // Poliwhirl — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    64: [
      // Kadabra — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    67: [
      // Machoke — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    70: [
      // Weepinbell — GRASS/POISON
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    75: [
      // Graveler — ROCK/GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    82: [
      // Magneton — ELECTRIC/STEEL
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    93: [
      // Haunter — GHOST/POISON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    112: [
      // Rhydon — GROUND/ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    113: [
      // Chansey — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600]
    ],
    117: [
      // Seadra — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    125: [
      // Electabuzz — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    126: [
      // Magmar — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    148: [
      // Dragonair — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    153: [
      // Bayleef — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Trop Kick", "GRASS", "PHYSICAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    156: [
      // Quilava — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    159: [
      // Croconaw — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    176: [
      // Togetic — FAIRY/FLYING
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    180: [
      // Flaaffy — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    183: [
      // Marill — WATER/FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    188: [
      // Skiploom — GRASS/FLYING
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    221: [
      // Piloswine — ICE/GROUND
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Ice Fang", "ICE", "PHYSICAL", 65],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    233: [
      // Porygon2 — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600]
    ],
    247: [
      // Pupitar — ROCK/GROUND
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    253: [
      // Grovyle — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Trop Kick", "GRASS", "PHYSICAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    256: [
      // Combusken — FIRE/FIGHTING
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    259: [
      // Marshtomp — WATER/GROUND
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    266: [
      // Silcoon — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    268: [
      // Cascoon — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    271: [
      // Lombre — WATER/GRASS
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    274: [
      // Nuzleaf — GRASS/DARK
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    281: [
      // Kirlia — PSYCHIC/FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    288: [
      // Vigoroth — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    294: [
      // Loudred — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    305: [
      // Lairon — STEEL/ROCK
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Mega Forge", "STEEL", "PHYSICAL", 600]
    ],
    315: [
      // Roselia — GRASS/POISON
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    329: [
      // Vibrava — GROUND/DRAGON
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    356: [
      // Dusclops — GHOST
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul-Stealing 7-Star Strike", "GHOST", "PHYSICAL", 600]
    ],
    364: [
      // Sealeo — WATER/ICE
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Powder Snow", "ICE", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    372: [
      // Shelgon — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    375: [
      // Metang — STEEL/PSYCHIC
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Mega Forge", "STEEL", "PHYSICAL", 600]
    ],
    388: [
      // Grotle — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Trop Kick", "GRASS", "PHYSICAL", 70],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    391: [
      // Monferno — FIRE/FIGHTING
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Wake-Up Slap", "FIGHTING", "PHYSICAL", 70],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    394: [
      // Prinplup — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    397: [
      // Staravia — NORMAL/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    404: [
      // Luxio — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Bolt Beak", "ELECTRIC", "PHYSICAL", 85],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    444: [
      // Gabite — DRAGON/GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    496: [
      // Servine — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Trop Kick", "GRASS", "PHYSICAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    499: [
      // Pignite — FIRE/FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Temper Flare", "FIRE", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Blast Burn", "FIRE", "PHYSICAL", 600]
    ],
    502: [
      // Dewott — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    520: [
      // Tranquill — NORMAL/FLYING
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    536: [
      // Palpitoad — WATER/GROUND
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Mud Shot", "GROUND", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    541: [
      // Swadloon — BUG/GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    544: [
      // Whirlipede — BUG/POISON
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    552: [
      // Krokorok — GROUND/DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    575: [
      // Gothorita — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    578: [
      // Duosion — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    603: [
      // Eelektrik — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    611: [
      // Fraxure — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    634: [
      // Zweilous — DARK/DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    662: [
      // Fletchinder — FIRE/FLYING
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Blast Burn", "FIRE", "PHYSICAL", 600]
    ],
    665: [
      // Spewpa — BUG
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    680: [
      // Doublade — STEEL/GHOST
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Mega Forge", "STEEL", "PHYSICAL", 600]
    ],
    705: [
      // Sliggoo — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    723: [
      // Dartrix — GRASS/FLYING
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    726: [
      // Torracat — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    729: [
      // Brionne — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    732: [
      // Trumbeak — NORMAL/FLYING
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    737: [
      // Charjabug — BUG/ELECTRIC
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    762: [
      // Steenee — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    783: [
      // Hakamo-o — DRAGON/FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    814: [
      // Raboot — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Temper Flare", "FIRE", "PHYSICAL", 75],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Blast Burn", "FIRE", "PHYSICAL", 600]
    ],
    958: [
      // Tinkatuff — FAIRY/STEEL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Let's Snuggle Forever", "FAIRY", "PHYSICAL", 600]
    ]
  };
  var GOLPES_OUTLAND_2026_10_15 = {
    2010: [
      // Magnetic Electabuzz — ELECTRIC · base Electabuzz (#125)
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    2011: [
      // Ancient Dragonair — DRAGON · base Dragonair (#148)
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    2015: [
      // Heavy Piloswine — ICE/GROUND · base Piloswine (#221)
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Ice Fang", "ICE", "PHYSICAL", 65],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    2032: [
      // Furious Magmar — FIRE · base Magmar (#126)
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    2033: [
      // Ancient Pupitar — ROCK/GROUND · base Pupitar (#247)
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ]
  };
  var GOLPES_ARTE_NOVA = {};
  var GOLPES_CONTRATO_2026_10_16 = {
    507: [
      // Herdier — NORMAL · meio da cadeia
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Crush Claw", "NORMAL", "PHYSICAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    583: [
      // Vanillish — ICE · meio da cadeia
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Powder Snow", "ICE", "SPECIAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Freezy Frost", "ICE", "SPECIAL", 90],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    600: [
      // Klang — STEEL · meio da cadeia
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Iron Head", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600]
    ],
    651: [
      // Quilladin — GRASS · meio da cadeia
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    654: [
      // Braixen — FIRE · meio da cadeia
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    657: [
      // Frogadier — WATER · meio da cadeia
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    811: [
      // Thwackey — GRASS · meio da cadeia
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    816: [
      // Sobble — WATER · 1º estágio
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120]
    ],
    817: [
      // Drizzile — WATER · meio da cadeia
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    822: [
      // Corvisquire — FLYING · meio da cadeia
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    825: [
      // Dottler — BUG/PSYCHIC · meio da cadeia
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    838: [
      // Carkol — ROCK/FIRE · meio da cadeia
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    857: [
      // Hattrem — PSYCHIC · meio da cadeia
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    860: [
      // Morgrem — DARK/FAIRY · meio da cadeia (a folha chegou em 17/10, depois da leva)
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    886: [
      // Drakloak — DRAGON/GHOST · meio da cadeia
      ["Shadow Punch", "GHOST", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Order Up", "DRAGON", "PHYSICAL", 80],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    891: [
      // Kubfu — FIGHTING · 1º estágio
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120]
    ],
    907: [
      // Floragato — GRASS · meio da cadeia
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    910: [
      // Crocalor — FIRE · meio da cadeia
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    912: [
      // Quaxly — WATER · 1º estágio
      ["Triple Dive", "WATER", "PHYSICAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120]
    ],
    913: [
      // Quaxwell — WATER · meio da cadeia
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    922: [
      // Pawmo — ELECTRIC/FIGHTING · meio da cadeia
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Zing Zap", "ELECTRIC", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    929: [
      // Dolliv — GRASS/NORMAL · meio da cadeia
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    933: [
      // Naclstack — ROCK · meio da cadeia
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    997: [
      // Arctibax — DRAGON/ICE · meio da cadeia
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Order Up", "DRAGON", "PHYSICAL", 80],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    1011: [
      // Dipplin — GRASS/DRAGON · 1º estágio
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120]
    ],
    4075: [
      // Alolan Graveler — ROCK/ELECTRIC · forma de Alola (#75) · meio da cadeia
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    5122: [
      // Galarian Mr. Mime — ICE/PSYCHIC · forma de Galar (#122) · meio da cadeia
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Powder Snow", "ICE", "SPECIAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    5264: [
      // Galarian Linoone — DARK/NORMAL · forma de Galar (#264) · meio da cadeia
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    6705: [
      // Hisuian Sliggoo — STEEL/DRAGON · forma de Hisui (#705) · meio da cadeia
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600]
    ]
    // --- A leva de 18/10/2026, pela mesma régua: Darmanitan, Hoopa, Sinistea, Polteageist e Palafin ganham
    // arte, e entram nove formas (as duas capas da Wormadam, Persian de Alola, Dialga e Palkia Origin,
    // Shaymin Sky, Darmanitan Zen e o de Galar, Black Kyurem). Todas foram refeitas depois e saíram daqui; o
    // Sinistea, o último, na "Rework Movesets 05-11".
  };
  var GOLPES_MEGAS_CONTRATO_2026_10_16 = {};
  var GOLPES_MEGAS_2026_10_21 = {};
  var GOLPES_MEGAS_2026_10_22 = {};
  var GOLPES_REWORK_2026_10_23 = {};
  var GOLPES_MEGAS_2026_10_23 = {};
  var GOLPES_REWORK_2026_10_26 = {
    3: [
      // Venusaur — GRASS/POISON
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Grass Pledge", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Verdant Storm", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    6: [
      // Charizard — FIRE/FLYING
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Burst", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Mind Blown", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    15: [
      // Beedrill — BUG/POISON
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["U-turn", "BUG", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    26: [
      // Raichu — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 72],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    36: [
      // Clefable — FAIRY
      ["Pixie Dust", "FAIRY", "SPECIAL", 30],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Disarming Voice", "FAIRY", "SPECIAL", 120],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    65: [
      // Alakazam — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Dream Eater", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Zen Headbutt", "PSYCHIC", "SPECIAL", 80],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    80: [
      // Slowbro — WATER/PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Psybeam", "PSYCHIC", "SPECIAL", 70],
      ["Expanding Force", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    94: [
      // Gengar — GHOST/POISON
      ["Shadow Punch", "GHOST", "SPECIAL", 25],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Hex", "GHOST", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Toxic Deluge", "POISON", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    115: [
      // Kangaskhan — NORMAL
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Fake Out", "NORMAL", "PHYSICAL", 40],
      ["Pound", "NORMAL", "PHYSICAL", 50],
      ["Stomp", "NORMAL", "PHYSICAL", 65],
      ["Headbutt", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Skull Bash", "NORMAL", "PHYSICAL", 120],
      ["Last Resort", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    121: [
      // Starmie — WATER/PSYCHIC
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Brine", "WATER", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    127: [
      // Pinsir — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Revenge", "BUG", "PHYSICAL", 70],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Bug Buzz", "BUG", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Megahorn", "BUG", "PHYSICAL", 600]
    ],
    130: [
      // Gyarados — WATER/FLYING
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Dive", "WATER", "PHYSICAL", 65],
      ["Bounce", "FLYING", "PHYSICAL", 75],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    142: [
      // Aerodactyl — ROCK/FLYING
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Throw", "ROCK", "PHYSICAL", 45],
      ["Rock Tomb", "ROCK", "PHYSICAL", 55],
      ["Smack Down", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Stone Edge", "ROCK", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    149: [
      // Dragonite — DRAGON/FLYING
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 55],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Wing Attack", "FLYING", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Aerial Ace", "FLYING", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    154: [
      // Meganium — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Magical Leaf", "GRASS", "SPECIAL", 60],
      ["Seed Bomb", "GRASS", "PHYSICAL", 75],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 85],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Solar Blade", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Verdant Storm", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    160: [
      // Feraligatr — WATER
      ["Aqua Jet", "WATER", "PHYSICAL", 35],
      ["Flip Turn", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Waterfall", "WATER", "PHYSICAL", 65],
      ["Liquidation", "WATER", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Tidal Surge", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    181: [
      // Ampharos — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Electroweb", "ELECTRIC", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    208: [
      // Steelix — STEEL/GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bone Club", "GROUND", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Iron Head", "STEEL", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["High Horsepower", "GROUND", "PHYSICAL", 95],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    212: [
      // Scizor — BUG/STEEL
      ["Twineedle", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Steamroller", "BUG", "PHYSICAL", 65],
      ["U-turn", "BUG", "PHYSICAL", 70],
      ["X-Scissor", "BUG", "PHYSICAL", 80],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    214: [
      // Heracross — BUG/FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 55],
      ["Counter", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Collision Course", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Megahorn", "BUG", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    227: [
      // Skarmory — STEEL/FLYING
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gyro Ball", "STEEL", "PHYSICAL", 55],
      ["Sky Drop", "FLYING", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    229: [
      // Houndoom — DARK/FIRE
      ["Payback", "DARK", "SPECIAL", 35],
      ["Thief", "DARK", "SPECIAL", 45],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Dark Pulse", "DARK", "SPECIAL", 65],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    248: [
      // Tyranitar — ROCK/DARK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Throw", "ROCK", "PHYSICAL", 45],
      ["Rock Tomb", "ROCK", "PHYSICAL", 55],
      ["Bite", "DARK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Stone Edge", "ROCK", "PHYSICAL", 100],
      ["Fling", "DARK", "PHYSICAL", 120],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    254: [
      // Sceptile — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Grass Knot", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Magical Leaf", "GRASS", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Grass Pledge", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Verdant Storm", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    260: [
      // Swampert — WATER/GROUND
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Yawn", "WATER", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 50],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Liquidation", "WATER", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Frost Power", "WATER", "PHYSICAL", 150],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    282: [
      // Gardevoir — PSYCHIC/FAIRY
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Lumina Crash", "PSYCHIC", "SPECIAL", 80],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    302: [
      // Sableye — DARK/GHOST
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Shadow Claw", "GHOST", "PHYSICAL", 55],
      ["Last Respects", "GHOST", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    303: [
      // Mawile — STEEL/FAIRY
      ["Disarming Voice", "FAIRY", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Draining Kiss", "FAIRY", "PHYSICAL", 50],
      ["Spirit Break", "FAIRY", "PHYSICAL", 65],
      ["Metal Burst", "STEEL", "PHYSICAL", 75],
      ["Dazzling Gleam", "FAIRY", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Fae Hammer", "FAIRY", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    306: [
      // Aggron — STEEL/ROCK
      ["Bullet Punch", "STEEL", "PHYSICAL", 35],
      ["Gear Grind", "STEEL", "PHYSICAL", 45],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Smart Strike", "STEEL", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Iron Tail", "STEEL", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    308: [
      // Medicham — FIGHTING/PSYCHIC
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Counter", "FIGHTING", "PHYSICAL", 40],
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 50],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Psybeam", "PSYCHIC", "SPECIAL", 70],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Aura Burst", "FIGHTING", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "SPECIAL", 600]
    ],
    310: [
      // Manectric — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    319: [
      // Sharpedo — WATER/DARK
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Flip Turn", "WATER", "PHYSICAL", 55],
      ["Waterfall", "WATER", "PHYSICAL", 65],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    334: [
      // Altaria — DRAGON/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Chatter", "FLYING", "SPECIAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Air Cutter", "FLYING", "SPECIAL", 70],
      ["Fly", "FLYING", "PHYSICAL", 85],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    354: [
      // Banette — GHOST
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Rage Fist", "GHOST", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Astral Barrage", "GHOST", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    358: [
      // Chimecho — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mega Mindbreak", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    359: [
      // Absol — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Bite", "DARK", "PHYSICAL", 45],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Thief", "DARK", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Throat Chop", "DARK", "PHYSICAL", 80],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Baddy Bad", "DARK", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    362: [
      // Glalie — ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Frost Breath", "ICE", "SPECIAL", 60],
      ["Sheer Cold", "ICE", "SPECIAL", 70],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Ice Spinner", "ICE", "PHYSICAL", 90],
      ["Blizzard", "ICE", "SPECIAL", 120],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    373: [
      // Salamence — DRAGON/FLYING
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 55],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Fly", "FLYING", "PHYSICAL", 75],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    376: [
      // Metagross — STEEL/PSYCHIC
      ["Confusion", "PSYCHIC", "PHYSICAL", 35],
      ["Twin Beam", "PSYCHIC", "PHYSICAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Zen Headbutt", "PSYCHIC", "PHYSICAL", 80],
      ["Psystrike", "PSYCHIC", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    398: [
      // Staraptor — NORMAL/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 28],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Pluck", "FLYING", "PHYSICAL", 70],
      ["Take Down", "NORMAL", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Endeavor", "NORMAL", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    428: [
      // Lopunny — NORMAL
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Cut", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Headbutt", "NORMAL", "PHYSICAL", 70],
      ["Strength", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    445: [
      // Garchomp — DRAGON/GROUND
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 55],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    460: [
      // Abomasnow — GRASS/ICE
      ["Icicle Spear", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Frost Breath", "ICE", "SPECIAL", 60],
      ["Razor Leaf", "GRASS", "PHYSICAL", 70],
      ["Ice Spinner", "ICE", "PHYSICAL", 80],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Ice Time", "ICE", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    475: [
      // Gallade — PSYCHIC/FIGHTING
      ["Stored Power", "PSYCHIC", "PHYSICAL", 25],
      ["Twin Beam", "PSYCHIC", "PHYSICAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Psyshock", "PSYCHIC", "PHYSICAL", 80],
      ["Psychic", "PSYCHIC", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    478: [
      // Froslass — ICE/GHOST
      ["Icicle Spear", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Frost Breath", "ICE", "SPECIAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Icicle Crash", "ICE", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Ice Time", "ICE", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    500: [
      // Emboar — FIRE/FIGHTING
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Heat Crash", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Temper Flare", "FIRE", "PHYSICAL", 90],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Superpower", "FIGHTING", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["V-create", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    530: [
      // Excadrill — GROUND/STEEL
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Mega Forge", "STEEL", "PHYSICAL", 600]
    ],
    531: [
      // Audino — NORMAL
      ["Tackle", "NORMAL", "PHYSICAL", 35],
      ["Pound", "NORMAL", "PHYSICAL", 40],
      ["Covet", "NORMAL", "PHYSICAL", 55],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Tri Attack", "NORMAL", "SPECIAL", 75],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    545: [
      // Scolipede — BUG/POISON
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["X-Scissor", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Mega Swarm", "BUG", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600]
    ],
    560: [
      // Scrafty — DARK/FIGHTING
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Power-Up Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Brick Break", "FIGHTING", "PHYSICAL", 65],
      ["Low Kick", "FIGHTING", "PHYSICAL", 70],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Beat Up", "DARK", "PHYSICAL", 120],
      ["High Jump Kick", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    609: [
      // Chandelure — GHOST/FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Burning Jealousy", "FIRE", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Fiery Dance", "FIRE", "SPECIAL", 80],
      ["Heat Wave", "FIRE", "SPECIAL", 95],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    623: [
      // Golurk — GROUND/GHOST
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bulldoze", "GROUND", "PHYSICAL", 45],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 65],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Soul-Stealing 7-Star Strike", "GHOST", "PHYSICAL", 600]
    ],
    652: [
      // Chesnaught — GRASS/FIGHTING
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Timber Crash", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    655: [
      // Delphox — FIRE/PSYCHIC
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Psychic", "PSYCHIC", "SPECIAL", 85],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Mind Blown", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    668: [
      // Pyroar — FIRE/NORMAL
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600]
    ],
    670: [
      // Floette — FAIRY · forma do meio (→ Florges #671), com o PAR de 600 que a planilha dá
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    678: [
      // Meowstic — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    687: [
      // Malamar — DARK/PSYCHIC
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Psychic Fangs", "PSYCHIC", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    689: [
      // Barbaracle — ROCK/WATER
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Smack Down", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    691: [
      // Dragalge — POISON/DRAGON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid", "POISON", "SPECIAL", 40],
      ["Acid Spray", "POISON", "SPECIAL", 50],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 75],
      ["Sludge Wave", "POISON", "SPECIAL", 85],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    701: [
      // Hawlucha — FIGHTING/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Acrobatics", "FLYING", "PHYSICAL", 70],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["High Jump Kick", "FIGHTING", "PHYSICAL", 120],
      ["Sky Attack", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    740: [
      // Crabominable — FIGHTING/ICE
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Ice Time", "ICE", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    768: [
      // Golisopod — BUG/WATER
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    780: [
      // Drampa — NORMAL/DRAGON
      ["Weather Ball", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Mega Cataclysm", "DRAGON", "SPECIAL", 600],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600]
    ],
    870: [
      // Falinks — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    952: [
      // Scovillain — GRASS/FIRE
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    978: [
      // Tatsugiri — DRAGON/WATER
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Dragon Breath", "DRAGON", "SPECIAL", 45],
      ["Dragon Darts", "DRAGON", "SPECIAL", 50],
      ["Fickle Beam", "DRAGON", "SPECIAL", 65],
      ["Snipe Shot", "WATER", "SPECIAL", 75],
      ["Core Enforcer", "DRAGON", "SPECIAL", 85],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600]
    ],
    998: [
      // Baxcalibur — DRAGON/ICE
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Icicle Crash", "ICE", "PHYSICAL", 75],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ]
  };
  var GOLPES_MEGAS_2026_10_26 = {};
  var GOLPES_REWORK_2026_10_27 = {
    12: [
      // Butterfree — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "SPECIAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    4020: [
      // Alolan Raticate — DARK/NORMAL
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Comeuppance", "DARK", "PHYSICAL", 80],
      ["Darkest Lariat", "DARK", "PHYSICAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    20: [
      // Raticate — NORMAL
      ["Bind", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Cut", "NORMAL", "PHYSICAL", 50],
      ["Facade", "NORMAL", "PHYSICAL", 65],
      ["Strength", "NORMAL", "PHYSICAL", 75],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Endeavor", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    22: [
      // Fearow — NORMAL/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 30],
      ["Peck", "FLYING", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Pluck", "FLYING", "PHYSICAL", 60],
      ["Wing Attack", "FLYING", "PHYSICAL", 70],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    4028: [
      // Alolan Sandslash — ICE/STEEL
      ["Ice Shard", "ICE", "PHYSICAL", 35],
      ["Icicle Spear", "ICE", "PHYSICAL", 40],
      ["Avalanche", "ICE", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Ice Fang", "ICE", "PHYSICAL", 70],
      ["Ice Spinner", "ICE", "PHYSICAL", 80],
      ["Icicle Crash", "ICE", "PHYSICAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    28: [
      // Sandslash — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Precipice Blades", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    31: [
      // Nidoqueen — POISON/GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    34: [
      // Nidoking — POISON/GROUND
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Precipice Blades", "GROUND", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600]
    ],
    4038: [
      // Alolan Ninetales — ICE/FAIRY
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Aurora Beam", "ICE", "SPECIAL", 55],
      ["Icy Wind", "ICE", "SPECIAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Ice Time", "ICE", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    40: [
      // Wigglytuff — NORMAL/FAIRY
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 45],
      ["Round", "NORMAL", "SPECIAL", 55],
      ["Tera Blast", "NORMAL", "SPECIAL", 65],
      ["Swift", "NORMAL", "SPECIAL", 70],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Disarming Voice", "FAIRY", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    45: [
      // Vileplume — GRASS/POISON
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Grass Knot", "GRASS", "SPECIAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 80],
      ["Magical Leaf", "GRASS", "SPECIAL", 96],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    47: [
      // Parasect — BUG/GRASS
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fell Stinger", "BUG", "PHYSICAL", 45],
      ["Fury Cutter", "BUG", "PHYSICAL", 55],
      ["X-Scissor", "BUG", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    49: [
      // Venomoth — BUG/POISON
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Struggle Bug", "BUG", "SPECIAL", 45],
      ["Silver Wind", "BUG", "SPECIAL", 55],
      ["Signal Beam", "BUG", "SPECIAL", 65],
      ["Bug Bite", "BUG", "SPECIAL", 75],
      ["Sludge Bomb", "POISON", "SPECIAL", 85],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    4051: [
      // Alolan Dugtrio — GROUND/STEEL
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    51: [
      // Dugtrio — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 50],
      ["Bone Club", "GROUND", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Precipice Blades", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    4053: [
      // Alolan Persian — DARK
      ["Power Trip", "DARK", "SPECIAL", 25],
      ["Fling", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Dark Pulse", "DARK", "SPECIAL", 65],
      ["Night Slash", "DARK", "SPECIAL", 70],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Pursuit", "DARK", "SPECIAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    55: [
      // Golduck — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Surf", "WATER", "SPECIAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    6059: [
      // Hisuian Arcanine — FIRE/ROCK
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Temper Flare", "FIRE", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Fire Fang", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    62: [
      // Poliwrath — WATER/FIGHTING
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 65],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    76: [
      // Golem — ROCK/GROUND
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Smack Down", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    5078: [
      // Galarian Rapidash — PSYCHIC/FAIRY
      ["Stored Power", "PSYCHIC", "PHYSICAL", 25],
      ["Twin Beam", "PSYCHIC", "PHYSICAL", 40],
      ["Mirror Coat", "PSYCHIC", "PHYSICAL", 55],
      ["Spirit Break", "FAIRY", "PHYSICAL", 65],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "PHYSICAL", 90],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    78: [
      // Rapidash — FIRE
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Fire Fang", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Pyro Ball", "FIRE", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    5080: [
      // Galarian Slowbro — POISON/PSYCHIC
      ["Acid Spray", "POISON", "SPECIAL", 35],
      ["Acid", "POISON", "SPECIAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Psyshock", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Toxic Torrent", "POISON", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    87: [
      // Dewgong — WATER/ICE
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    4089: [
      // Alolan Muk — POISON/DARK
      ["Poison Fang", "POISON", "PHYSICAL", 35],
      ["Mortal Spin", "POISON", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Poison Jab", "POISON", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    91: [
      // Cloyster — WATER/ICE
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Razor Shell", "WATER", "PHYSICAL", 65],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    99: [
      // Kingler — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    101: [
      // Electrode — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    6101: [
      // Hisuian Electrode — ELECTRIC/GRASS
      ["Charge Beam", "ELECTRIC", "SPECIAL", 35],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    4103: [
      // Alolan Exeggutor — GRASS/DRAGON
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Grass Knot", "GRASS", "SPECIAL", 85],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    103: [
      // Exeggutor — GRASS/PSYCHIC
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Leaf Tornado", "GRASS", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    4105: [
      // Alolan Marowak — FIRE/GHOST
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Fire Fang", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Fire Punch", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    105: [
      // Marowak — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bone Club", "GROUND", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    122: [
      // Mr. Mime — PSYCHIC/FAIRY
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Confusion", "PSYCHIC", "SPECIAL", 45],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Disarming Voice", "FAIRY", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    7128: [
      // Paldean Tauros — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    9128: [
      // Paldean Tauros Aqua — FIGHTING/WATER
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    8128: [
      // Paldean Tauros Blaze — FIGHTING/FIRE
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    131: [
      // Lapras — WATER/ICE
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Waterfall", "WATER", "PHYSICAL", 65],
      ["Avalanche", "ICE", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    132: [
      // Ditto — NORMAL
      ["Tail Slap", "NORMAL", "PHYSICAL", 25],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    139: [
      // Omastar — ROCK/WATER
      ["Rock Blast", "ROCK", "SPECIAL", 25],
      ["Rollout", "ROCK", "SPECIAL", 40],
      ["Rock Throw", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Rock Slide", "ROCK", "SPECIAL", 80],
      ["Diamond Storm", "ROCK", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600]
    ],
    141: [
      // Kabutops — ROCK/WATER
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Rock Slide", "ROCK", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    143: [
      // Snorlax — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    6157: [
      // Hisuian Typhlosion — FIRE/GHOST
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Mind Blown", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    162: [
      // Furret — NORMAL
      ["Bind", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Facade", "NORMAL", "PHYSICAL", 55],
      ["Super Fang", "NORMAL", "PHYSICAL", 65],
      ["Body Slam", "NORMAL", "PHYSICAL", 75],
      ["Slam", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Endeavor", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    164: [
      // Noctowl — NORMAL/FLYING
      ["Weather Ball", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600]
    ],
    166: [
      // Ledian — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "SPECIAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    168: [
      // Ariados — BUG/POISON
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Mega Swarm", "BUG", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600]
    ],
    171: [
      // Lanturn — WATER/ELECTRIC
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    178: [
      // Xatu — PSYCHIC/FLYING
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Zen Headbutt", "PSYCHIC", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    184: [
      // Azumarill — WATER/FAIRY
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Surf", "WATER", "SPECIAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    185: [
      // Sudowoodo — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rollout", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Meteor Beam", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 600]
    ],
    186: [
      // Politoed — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Surf", "WATER", "SPECIAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    189: [
      // Jumpluff — GRASS/FLYING
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Energy Ball", "GRASS", "SPECIAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    192: [
      // Sunflora — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Energy Ball", "GRASS", "SPECIAL", 85],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    195: [
      // Quagsire — WATER/GROUND
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    5199: [
      // Galarian Slowking — POISON/PSYCHIC
      ["Acid", "POISON", "SPECIAL", 35],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Venoshock", "POISON", "SPECIAL", 70],
      ["Sludge Wave", "POISON", "SPECIAL", 85],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Toxic Torrent", "POISON", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    199: [
      // Slowking — WATER/PSYCHIC
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Scald", "WATER", "SPECIAL", 75],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    205: [
      // Forretress — BUG/STEEL
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Mega Forge", "STEEL", "PHYSICAL", 600],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    210: [
      // Granbull — FAIRY
      ["Disarming Voice", "FAIRY", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Draining Kiss", "FAIRY", "PHYSICAL", 50],
      ["Dazzling Gleam", "FAIRY", "PHYSICAL", 65],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Alluring Voice", "FAIRY", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Fleur Cannon", "FAIRY", "PHYSICAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Fae Hammer", "FAIRY", "PHYSICAL", 150],
      ["Let's Snuggle Forever", "FAIRY", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    213: [
      // Shuckle — BUG/ROCK
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Silver Wind", "BUG", "SPECIAL", 55],
      ["Struggle Bug", "BUG", "SPECIAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    219: [
      // Magcargo — FIRE/ROCK
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600]
    ],
    224: [
      // Octillery — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    225: [
      // Delibird — ICE/FLYING
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Ice Beam", "ICE", "SPECIAL", 85],
      ["Blizzard", "ICE", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Glacial Lance", "ICE", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    226: [
      // Mantine — WATER/FLYING
      ["Bubble", "WATER", "SPECIAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    232: [
      // Donphan — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Bulldoze", "GROUND", "PHYSICAL", 45],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bone Club", "GROUND", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Precipice Blades", "GROUND", "PHYSICAL", 600]
    ],
    235: [
      // Smeargle — NORMAL
      ["Bind", "NORMAL", "PHYSICAL", 25],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Scratch", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    241: [
      // Miltank — NORMAL
      ["Bind", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Headbutt", "NORMAL", "PHYSICAL", 55],
      ["Stomp", "NORMAL", "PHYSICAL", 65],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Strength", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Endeavor", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    242: [
      // Blissey — NORMAL
      ["Terrain Pulse", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Weather Ball", "NORMAL", "SPECIAL", 50],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Tera Blast", "NORMAL", "SPECIAL", 75],
      ["Hyper Voice", "NORMAL", "SPECIAL", 85],
      ["Swift", "NORMAL", "SPECIAL", 100],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Tera Starstorm", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Giga Impact", "NORMAL", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    262: [
      // Mightyena — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Baddy Bad", "DARK", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    264: [
      // Linoone — NORMAL · evolui (→ #862)
      ["Fury Swipes", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Cut", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Headbutt", "NORMAL", "PHYSICAL", 70],
      ["Slash", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Flail", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600]
    ],
    267: [
      // Beautifly — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Leech Life", "BUG", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    269: [
      // Dustox — BUG/POISON
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Venoshock", "POISON", "SPECIAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    272: [
      // Ludicolo — WATER/GRASS
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    279: [
      // Pelipper — WATER/FLYING
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    284: [
      // Masquerain — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Signal Beam", "BUG", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Leech Life", "BUG", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    286: [
      // Breloom — GRASS/FIGHTING
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    289: [
      // Slaking — NORMAL
      ["Bind", "NORMAL", "PHYSICAL", 25],
      ["Scratch", "NORMAL", "PHYSICAL", 40],
      ["Slash", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Flail", "NORMAL", "PHYSICAL", 120],
      ["Mega Kick", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    291: [
      // Ninjask — BUG/FLYING
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Mega Swarm", "BUG", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    295: [
      // Exploud — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Uproar", "NORMAL", "SPECIAL", 85],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    311: [
      // Plusle — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Electroweb", "ELECTRIC", "SPECIAL", 45],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunder", "ELECTRIC", "SPECIAL", 100],
      ["Electro Shot", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    312: [
      // Minun — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    313: [
      // Volbeat — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Bug Buzz", "BUG", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Megahorn", "BUG", "PHYSICAL", 600]
    ],
    314: [
      // Illumise — BUG
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "SPECIAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Leech Life", "BUG", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Attack Order", "BUG", "SPECIAL", 120],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Mega Swarm", "BUG", "SPECIAL", 600]
    ],
    321: [
      // Wailord — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    324: [
      // Torkoal — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    326: [
      // Grumpig — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mega Mindbreak", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    330: [
      // Flygon — GROUND/DRAGON
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    337: [
      // Lunatone — ROCK/PSYCHIC
      ["Rock Blast", "ROCK", "SPECIAL", 25],
      ["Salt Cure", "ROCK", "SPECIAL", 40],
      ["Rock Throw", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Slide", "ROCK", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Diamond Storm", "ROCK", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600]
    ],
    338: [
      // Solrock — ROCK/PSYCHIC
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Psychic Fangs", "PSYCHIC", "PHYSICAL", 600]
    ],
    342: [
      // Crawdaunt — WATER/DARK
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    344: [
      // Claydol — GROUND/PSYCHIC
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Mud-Slap", "GROUND", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Scorching Sands", "GROUND", "SPECIAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Seismic Blast", "GROUND", "SPECIAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    348: [
      // Armaldo — ROCK/BUG
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    351: [
      // Castform — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Weather Ball", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    8351: [
      // Castform Rainy — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Water Pulse", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Scald", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    9351: [
      // Castform Snowy — ICE
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    11351: [
      // Castform Sunny — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Fire Blast", "FIRE", "SPECIAL", 120],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    352: [
      // Kecleon — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    365: [
      // Walrein — WATER/ICE
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Ice Beam", "ICE", "SPECIAL", 85],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Blizzard", "ICE", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Glacial Lance", "ICE", "SPECIAL", 600]
    ],
    367: [
      // Huntail — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    368: [
      // Gorebyss — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Mega Maelstrom", "WATER", "SPECIAL", 600]
    ],
    369: [
      // Relicanth — WATER/ROCK
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    370: [
      // Luvdisc — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Mega Maelstrom", "WATER", "SPECIAL", 600]
    ],
    402: [
      // Kricketune — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Megahorn", "BUG", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    405: [
      // Luxray — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Bolt Beak", "ELECTRIC", "PHYSICAL", 85],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    409: [
      // Rampardos — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Mega Landslide", "ROCK", "PHYSICAL", 600]
    ],
    413: [
      // Wormadam — BUG/GRASS
      ["Struggle Bug", "BUG", "SPECIAL", 35],
      ["Infestation", "BUG", "SPECIAL", 40],
      ["Silver Wind", "BUG", "SPECIAL", 55],
      ["Signal Beam", "BUG", "SPECIAL", 65],
      ["Energy Ball", "GRASS", "SPECIAL", 75],
      ["Pollen Puff", "BUG", "SPECIAL", 85],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Attack Order", "BUG", "SPECIAL", 120],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    8413: [
      // Wormadam Sandy — BUG/GROUND
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    9413: [
      // Wormadam Trash — BUG/STEEL
      ["Fury Cutter", "BUG", "PHYSICAL", 35],
      ["Struggle Bug", "BUG", "SPECIAL", 45],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Flash Cannon", "STEEL", "SPECIAL", 65],
      ["Silver Wind", "BUG", "SPECIAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    414: [
      // Mothim — BUG/FLYING
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["U-turn", "BUG", "PHYSICAL", 70],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    416: [
      // Vespiquen — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    417: [
      // Pachirisu — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    419: [
      // Floatzel — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    423: [
      // Gastrodon — WATER/GROUND
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    424: [
      // Ambipom — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    429: [
      // Mismagius — GHOST
      ["Lick", "GHOST", "SPECIAL", 30],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Hex", "GHOST", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Bitter Malice", "GHOST", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Mega Eclipse", "GHOST", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    430: [
      // Honchkrow — DARK/FLYING
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Thief", "DARK", "PHYSICAL", 45],
      ["Night Slash", "DARK", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Mega Abyss", "DARK", "PHYSICAL", 600]
    ],
    435: [
      // Skuntank — POISON/DARK
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Poison Jab", "POISON", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    442: [
      // Spiritomb — GHOST/DARK
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Shadow Ball", "GHOST", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Mega Abyss", "DARK", "SPECIAL", 600],
      ["Soul-Stealing 7-Star Strike", "GHOST", "PHYSICAL", 600]
    ],
    450: [
      // Hippowdon — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    452: [
      // Drapion — POISON/DARK
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Poison Jab", "POISON", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    454: [
      // Toxicroak — POISON/FIGHTING
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600]
    ],
    457: [
      // Lumineon — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    461: [
      // Weavile — DARK/ICE
      ["Icicle Spear", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Ice Spinner", "ICE", "PHYSICAL", 80],
      ["Blizzard", "ICE", "PHYSICAL", 100],
      ["Beat Up", "DARK", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    462: [
      // Magnezone — ELECTRIC/STEEL
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    464: [
      // Rhyperior — GROUND/ROCK
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    466: [
      // Electivire — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 55],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Zing Zap", "ELECTRIC", "PHYSICAL", 80],
      ["Supercell Slam", "ELECTRIC", "PHYSICAL", 100],
      ["Aura Wheel", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    472: [
      // Gliscor — GROUND/FLYING
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    473: [
      // Mamoswine — ICE/GROUND
      ["Icicle Spear", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Avalanche", "ICE", "PHYSICAL", 55],
      ["Ice Fang", "ICE", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    476: [
      // Probopass — ROCK/STEEL
      ["Rock Blast", "ROCK", "SPECIAL", 25],
      ["Salt Cure", "ROCK", "SPECIAL", 40],
      ["Smack Down", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Slide", "ROCK", "SPECIAL", 75],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Diamond Storm", "ROCK", "SPECIAL", 100],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600]
    ],
    12479: [
      // Fan Rotom — ELECTRIC/FLYING
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    11479: [
      // Frost Rotom — ELECTRIC/ICE
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Ice Time", "ICE", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    8479: [
      // Heat Rotom — ELECTRIC/FIRE
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    1479: [
      // Mow Rotom — ELECTRIC/GRASS
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    479: [
      // Rotom — ELECTRIC/GHOST
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    9479: [
      // Wash Rotom — ELECTRIC/WATER
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    508: [
      // Stoutland — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    510: [
      // Liepard — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Dark Pulse", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Mega Abyss", "DARK", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    516: [
      // Simipour — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    518: [
      // Musharna — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mega Mindbreak", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    521: [
      // Unfezant — NORMAL/FLYING
      ["Quick Attack", "NORMAL", "PHYSICAL", 35],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Fly", "FLYING", "PHYSICAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Sky Attack", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    523: [
      // Zebstrika — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Bolt Beak", "ELECTRIC", "PHYSICAL", 85],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    528: [
      // Swoobat — PSYCHIC/FLYING
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    547: [
      // Whimsicott — GRASS/FAIRY
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Absorb", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Razor Leaf", "GRASS", "PHYSICAL", 60],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    6549: [
      // Hisuian Lilligant — GRASS/FIGHTING
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    549: [
      // Lilligant — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Leaf Tornado", "GRASS", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Mega Bloom", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    553: [
      // Krookodile — GROUND/DARK
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    555: [
      // Darmanitan — FIRE
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Fire Fang", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    8555: [
      // Darmanitan Zen — FIRE/PSYCHIC
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    5555: [
      // Galarian Darmanitan — ICE
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    9555: [
      // Galarian Darmanitan Zen — ICE/FIRE
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Icicle Spear", "ICE", "PHYSICAL", 40],
      ["Ice Shard", "ICE", "PHYSICAL", 50],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Fang", "ICE", "PHYSICAL", 70],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Icicle Crash", "ICE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Ice Time", "ICE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    561: [
      // Sigilyph — PSYCHIC/FLYING
      ["Gust", "FLYING", "SPECIAL", 35],
      ["Air Cutter", "FLYING", "SPECIAL", 45],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Chatter", "FLYING", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Tempest Gale", "FLYING", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    563: [
      // Cofagrigus — GHOST
      ["Lick", "GHOST", "SPECIAL", 30],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Hex", "GHOST", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Bitter Malice", "GHOST", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Mega Eclipse", "GHOST", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    565: [
      // Carracosta — WATER/ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Tomb", "ROCK", "PHYSICAL", 45],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    567: [
      // Archeops — ROCK/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Pluck", "FLYING", "PHYSICAL", 45],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    6571: [
      // Hisuian Zoroark — NORMAL/GHOST
      ["Terrain Pulse", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Round", "NORMAL", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Tera Blast", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    571: [
      // Zoroark — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Dark Pulse", "DARK", "SPECIAL", 80],
      ["Night Daze", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Mega Abyss", "DARK", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    573: [
      // Cinccino — NORMAL
      ["Tail Slap", "NORMAL", "PHYSICAL", 25],
      ["Pound", "NORMAL", "PHYSICAL", 40],
      ["Super Fang", "NORMAL", "PHYSICAL", 55],
      ["Slam", "NORMAL", "PHYSICAL", 65],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Take Down", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Endeavor", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    576: [
      // Gothitelle — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mega Mindbreak", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    579: [
      // Reuniclus — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mega Mindbreak", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    581: [
      // Swanna — WATER/FLYING
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    584: [
      // Vanilluxe — ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Ice Beam", "ICE", "SPECIAL", 85],
      ["Freezy Frost", "ICE", "SPECIAL", 90],
      ["Blizzard", "ICE", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Glacial Lance", "ICE", "SPECIAL", 600],
      ["Mega Permafrost", "ICE", "SPECIAL", 600]
    ],
    587: [
      // Emolga — ELECTRIC/FLYING
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Wing Attack", "FLYING", "PHYSICAL", 70],
      ["Spark", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mega Thunderstorm", "ELECTRIC", "PHYSICAL", 600]
    ],
    593: [
      // Jellicent — WATER/GHOST
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Brine", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Scald", "WATER", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    594: [
      // Alomomola — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    596: [
      // Galvantula — BUG/ELECTRIC
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Signal Beam", "BUG", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Leech Life", "BUG", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    606: [
      // Beheeyem — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    614: [
      // Beartic — ICE
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Avalanche", "ICE", "PHYSICAL", 55],
      ["Ice Fang", "ICE", "PHYSICAL", 65],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Mega Permafrost", "ICE", "PHYSICAL", 600]
    ],
    615: [
      // Cryogonal — ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Blizzard", "ICE", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Glacial Lance", "ICE", "SPECIAL", 600],
      ["Ice Burn", "ICE", "SPECIAL", 600]
    ],
    617: [
      // Accelgor — BUG
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "SPECIAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Leech Life", "BUG", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Attack Order", "BUG", "SPECIAL", 120],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Mega Swarm", "BUG", "SPECIAL", 600]
    ],
    5618: [
      // Galarian Stunfisk — GROUND/STEEL
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    618: [
      // Stunfisk — GROUND/ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Discharge", "ELECTRIC", "SPECIAL", 75],
      ["Scorching Sands", "GROUND", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Land's Wrath", "GROUND", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    6628: [
      // Hisuian Braviary — PSYCHIC/FLYING
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Esper Wing", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    632: [
      // Durant — BUG/STEEL
      ["Fury Cutter", "BUG", "PHYSICAL", 35],
      ["Bug Bite", "BUG", "PHYSICAL", 45],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["X-Scissor", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    637: [
      // Volcarona — BUG/FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Flamethrower", "FIRE", "SPECIAL", 75],
      ["Fiery Dance", "FIRE", "SPECIAL", 80],
      ["Heat Wave", "FIRE", "SPECIAL", 95],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    666: [
      // Vivillon — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Leech Life", "BUG", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    671: [
      // Florges — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Mega Radiance", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    675: [
      // Pangoro — FIGHTING/DARK
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    676: [
      // Furfrou — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    681: [
      // Aegislash — STEEL/GHOST · evolui (→ #8681)
      ["Shadow Punch", "GHOST", "PHYSICAL", 25],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Ominous Wind", "GHOST", "SPECIAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Shadow Ball", "GHOST", "SPECIAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    8681: [
      // Aegislash Blade — STEEL/GHOST
      ["Shadow Punch", "GHOST", "PHYSICAL", 25],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Ominous Wind", "GHOST", "SPECIAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Shadow Ball", "GHOST", "SPECIAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    683: [
      // Aromatisse — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Mega Radiance", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    693: [
      // Clawitzer — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Mega Maelstrom", "WATER", "SPECIAL", 600]
    ],
    695: [
      // Heliolisk — ELECTRIC/NORMAL
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    697: [
      // Tyrantrum — ROCK/DRAGON
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Diamond Storm", "ROCK", "PHYSICAL", 85],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    699: [
      // Aurorus — ROCK/ICE
      ["Rock Blast", "ROCK", "SPECIAL", 25],
      ["Ancient Power", "ROCK", "SPECIAL", 45],
      ["Rock Throw", "ROCK", "SPECIAL", 50],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Rock Slide", "ROCK", "SPECIAL", 75],
      ["Mighty Cleave", "ROCK", "SPECIAL", 85],
      ["Diamond Storm", "ROCK", "SPECIAL", 100],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Ice Burn", "ICE", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600]
    ],
    700: [
      // Sylveon — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Mega Radiance", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    702: [
      // Dedenne — ELECTRIC/FAIRY
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 75],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    703: [
      // Carbink — ROCK/FAIRY
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Smack Down", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Power Gem", "ROCK", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Meteor Beam", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600]
    ],
    707: [
      // Klefki — STEEL/FAIRY
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Flash Cannon", "STEEL", "SPECIAL", 65],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 75],
      ["Mirror Shot", "STEEL", "SPECIAL", 80],
      ["Iron Tail", "STEEL", "PHYSICAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Doom Desire", "STEEL", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    709: [
      // Trevenant — GHOST/GRASS
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Rage Fist", "GHOST", "PHYSICAL", 50],
      ["Last Respects", "GHOST", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Soul-Stealing 7-Star Strike", "GHOST", "PHYSICAL", 600]
    ],
    711: [
      // Gourgeist — GHOST/GRASS
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Rage Fist", "GHOST", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    713: [
      // Avalugg — ICE
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Avalanche", "ICE", "PHYSICAL", 55],
      ["Ice Fang", "ICE", "PHYSICAL", 65],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Mega Permafrost", "ICE", "PHYSICAL", 600]
    ],
    6713: [
      // Hisuian Avalugg — ICE/ROCK
      ["Icicle Spear", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    715: [
      // Noivern — FLYING/DRAGON
      ["Gust", "FLYING", "SPECIAL", 35],
      ["Air Cutter", "FLYING", "SPECIAL", 45],
      ["Chatter", "FLYING", "SPECIAL", 55],
      ["Oblivion Wing", "FLYING", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mega Cataclysm", "DRAGON", "SPECIAL", 600]
    ],
    6724: [
      // Hisuian Decidueye — GRASS/FIGHTING
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Timber Crash", "GRASS", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    733: [
      // Toucannon — NORMAL/FLYING
      ["Fury Attack", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Sky Attack", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    738: [
      // Vikavolt — BUG/ELECTRIC
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Signal Beam", "BUG", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Leech Life", "BUG", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600]
    ],
    741: [
      // Oricorio — FIRE/FLYING
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Fiery Dance", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    9741: [
      // Oricorio Pa'u — PSYCHIC/FLYING
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Eerie Spell", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    8741: [
      // Oricorio Pom-Pom — ELECTRIC/FLYING
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    11741: [
      // Oricorio Sensu — GHOST/FLYING
      ["Astonish", "GHOST", "SPECIAL", 30],
      ["Infernal Parade", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Hex", "GHOST", "SPECIAL", 80],
      ["Shadow Ball", "GHOST", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    743: [
      // Ribombee — BUG/FAIRY
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "SPECIAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    745: [
      // Lycanroc — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Stone Axe", "ROCK", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    9745: [
      // Lycanroc Dusk — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Stone Axe", "ROCK", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    8745: [
      // Lycanroc Midnight — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Stone Axe", "ROCK", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    746: [
      // Wishiwashi — WATER · evolui (→ #8746)
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    8746: [
      // Wishiwashi School — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Waterfall", "WATER", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    748: [
      // Toxapex — POISON/WATER
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    750: [
      // Mudsdale — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    752: [
      // Araquanid — WATER/BUG
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    754: [
      // Lurantis — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    756: [
      // Shiinotic — GRASS/FAIRY
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Leaf Tornado", "GRASS", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    758: [
      // Salazzle — POISON/FIRE
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Sludge Bomb", "POISON", "SPECIAL", 85],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Toxic Torrent", "POISON", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    764: [
      // Comfey — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Strange Steam", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    770: [
      // Palossand — GHOST/GROUND
      ["Astonish", "GHOST", "SPECIAL", 30],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Mud Shot", "GROUND", "SPECIAL", 60],
      ["Hex", "GHOST", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Bitter Malice", "GHOST", "SPECIAL", 90],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    771: [
      // Pyukumuku — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    776: [
      // Turtonator — FIRE/DRAGON
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Heat Wave", "FIRE", "SPECIAL", 85],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    777: [
      // Togedemaru — ELECTRIC/STEEL
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Mega Forge", "STEEL", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    778: [
      // Mimikyu — GHOST/FAIRY
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Rage Fist", "GHOST", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Play Rough", "FAIRY", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Let's Snuggle Forever", "FAIRY", "PHYSICAL", 600],
      ["Soul-Stealing 7-Star Strike", "GHOST", "PHYSICAL", 600]
    ],
    779: [
      // Bruxish — WATER/PSYCHIC
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Psychic Fangs", "PSYCHIC", "PHYSICAL", 600]
    ],
    781: [
      // Dhelmise — GHOST/GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    812: [
      // Rillaboom — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Timber Crash", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    818: [
      // Inteleon — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    820: [
      // Greedent — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    823: [
      // Corviknight — FLYING/STEEL
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 35],
      ["Peck", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    826: [
      // Orbeetle — BUG/PSYCHIC
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "SPECIAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    828: [
      // Thievul — DARK
      ["Power Trip", "DARK", "SPECIAL", 25],
      ["Fling", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Dark Pulse", "DARK", "SPECIAL", 65],
      ["Night Slash", "DARK", "SPECIAL", 70],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Pursuit", "DARK", "SPECIAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    830: [
      // Eldegoss — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Magical Leaf", "GRASS", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    832: [
      // Dubwool — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    834: [
      // Drednaw — WATER/ROCK
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    836: [
      // Boltund — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    841: [
      // Flapple — GRASS/DRAGON
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Grav Apple", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    842: [
      // Appletun — GRASS/DRAGON
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    844: [
      // Sandaconda — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bone Club", "GROUND", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    845: [
      // Cramorant — FLYING/WATER
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Air Cutter", "FLYING", "SPECIAL", 70],
      ["Air Slash", "FLYING", "SPECIAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tempest Gale", "FLYING", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    847: [
      // Barraskewda — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    849: [
      // Toxtricity — ELECTRIC/POISON
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    851: [
      // Centiskorch — FIRE/BUG
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Fire Fang", "FIRE", "PHYSICAL", 70],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    853: [
      // Grapploct — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    855: [
      // Polteageist — GHOST
      ["Shadow Punch", "GHOST", "SPECIAL", 25],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Hex", "GHOST", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Bitter Malice", "GHOST", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    858: [
      // Hatterene — PSYCHIC/FAIRY
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    861: [
      // Grimmsnarl — DARK/FAIRY
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Spirit Break", "FAIRY", "PHYSICAL", 65],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    862: [
      // Obstagoon — DARK/NORMAL
      ["Payback", "DARK", "PHYSICAL", 35],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Throat Chop", "DARK", "PHYSICAL", 80],
      ["Darkest Lariat", "DARK", "PHYSICAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    863: [
      // Perrserker — STEEL
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    864: [
      // Cursola — GHOST
      ["Shadow Punch", "GHOST", "SPECIAL", 25],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Hex", "GHOST", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Bitter Malice", "GHOST", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    865: [
      // Sirfetch'd — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    866: [
      // Mr. Rime — ICE/PSYCHIC
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Ice Time", "ICE", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    867: [
      // Runerigus — GROUND/GHOST
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Earthquake", "GROUND", "PHYSICAL", 85],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    869: [
      // Alcremie — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Mega Radiance", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    873: [
      // Frosmoth — ICE/BUG
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Aurora Beam", "ICE", "SPECIAL", 80],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    875: [
      // Eiscue — ICE
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    876: [
      // Indeedee — PSYCHIC/NORMAL
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Expanding Force", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    877: [
      // Morpeko — ELECTRIC/DARK
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Spark", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    879: [
      // Copperajah — STEEL
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    880: [
      // Dracozolt — ELECTRIC/DRAGON
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 55],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Bolt Beak", "ELECTRIC", "PHYSICAL", 85],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    882: [
      // Dracovish — WATER/DRAGON
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Fishious Rend", "WATER", "PHYSICAL", 85],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    899: [
      // Wyrdeer — NORMAL/PSYCHIC
      ["Tackle", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 65],
      ["Uproar", "NORMAL", "SPECIAL", 75],
      ["Tera Blast", "NORMAL", "SPECIAL", 80],
      ["Swift", "NORMAL", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    900: [
      // Kleavor — BUG/ROCK
      ["Pounce", "BUG", "PHYSICAL", 35],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    8901: [
      // Bloodmoon Ursaluna — GROUND/NORMAL
      ["Mud-Slap", "GROUND", "SPECIAL", 25],
      ["Mud Shot", "GROUND", "SPECIAL", 45],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Mud Bomb", "GROUND", "SPECIAL", 65],
      ["Scorching Sands", "GROUND", "SPECIAL", 70],
      ["Drill Run", "GROUND", "SPECIAL", 80],
      ["Thousand Waves", "GROUND", "SPECIAL", 90],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Seismic Blast", "GROUND", "SPECIAL", 150],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    901: [
      // Ursaluna — GROUND/NORMAL
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    902: [
      // Basculegion — WATER/GHOST
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    8902: [
      // Basculegion Female — WATER/GHOST
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Surf", "WATER", "SPECIAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    903: [
      // Sneasler — FIGHTING/POISON
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Brick Break", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    904: [
      // Overqwil — DARK/POISON
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Darkest Lariat", "DARK", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    908: [
      // Meowscarada — GRASS/DARK
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Timber Crash", "GRASS", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    911: [
      // Skeledirge — FIRE/GHOST
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Mind Blown", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    914: [
      // Quaquaval — WATER/FIGHTING
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Frost Power", "WATER", "PHYSICAL", 150],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    916: [
      // Oinkologne — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    918: [
      // Spidops — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Bug Buzz", "BUG", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Megahorn", "BUG", "PHYSICAL", 600]
    ],
    920: [
      // Lokix — BUG/DARK
      ["Pounce", "BUG", "PHYSICAL", 35],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    923: [
      // Pawmot — ELECTRIC/FIGHTING
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Spark", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    925: [
      // Maushold — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    927: [
      // Dachsbun — FAIRY
      ["Disarming Voice", "FAIRY", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Draining Kiss", "FAIRY", "PHYSICAL", 50],
      ["Dazzling Gleam", "FAIRY", "PHYSICAL", 65],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Alluring Voice", "FAIRY", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Fleur Cannon", "FAIRY", "PHYSICAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Fae Hammer", "FAIRY", "PHYSICAL", 150],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    930: [
      // Arboliva — GRASS/NORMAL
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Grass Knot", "GRASS", "SPECIAL", 85],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    931: [
      // Squawkabilly — NORMAL/FLYING
      ["Fury Attack", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Headbutt", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    934: [
      // Garganacl — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Stone Axe", "ROCK", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    936: [
      // Armarouge — FIRE/PSYCHIC
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    939: [
      // Bellibolt — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    941: [
      // Kilowattrel — ELECTRIC/FLYING
      ["Charge Beam", "ELECTRIC", "SPECIAL", 35],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    943: [
      // Mabosstiff — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Darkest Lariat", "DARK", "PHYSICAL", 90],
      ["Baddy Bad", "DARK", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    945: [
      // Grafaiai — POISON/NORMAL
      ["Poison Fang", "POISON", "PHYSICAL", 35],
      ["Mortal Spin", "POISON", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Poison Jab", "POISON", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    947: [
      // Brambleghast — GRASS/GHOST
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    950: [
      // Klawf — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Stone Axe", "ROCK", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    954: [
      // Rabsca — BUG/PSYCHIC
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Pounce", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    956: [
      // Espathra — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    959: [
      // Tinkaton — FAIRY/STEEL
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Spirit Break", "FAIRY", "PHYSICAL", 65],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Fae Hammer", "FAIRY", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    961: [
      // Wugtrio — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    962: [
      // Bombirdier — FLYING/DARK
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    964: [
      // Palafin — WATER · evolui (→ #8964)
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    8964: [
      // Palafin Hero — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    966: [
      // Revavroom — STEEL/POISON
      ["Gear Grind", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gyro Ball", "STEEL", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Iron Head", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    968: [
      // Orthworm — STEEL
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    970: [
      // Glimmora — ROCK/POISON
      ["Rock Blast", "ROCK", "SPECIAL", 25],
      ["Salt Cure", "ROCK", "SPECIAL", 40],
      ["Rock Throw", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Slide", "ROCK", "SPECIAL", 75],
      ["Sludge Wave", "POISON", "SPECIAL", 85],
      ["Diamond Storm", "ROCK", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    972: [
      // Houndstone — GHOST
      ["Shadow Punch", "GHOST", "PHYSICAL", 25],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Rage Fist", "GHOST", "PHYSICAL", 50],
      ["Last Respects", "GHOST", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Astral Barrage", "GHOST", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    973: [
      // Flamigo — FLYING/FIGHTING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Submission", "FIGHTING", "PHYSICAL", 75],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    975: [
      // Cetitan — ICE
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    976: [
      // Veluza — WATER/PSYCHIC
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 65],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    977: [
      // Dondozo — WATER
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    980: [
      // Clodsire — POISON/GROUND
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Poison Jab", "POISON", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    981: [
      // Farigiraf — NORMAL/PSYCHIC
      ["Terrain Pulse", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Weather Ball", "NORMAL", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    982: [
      // Dudunsparce — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    983: [
      // Kingambit — DARK/STEEL
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Kowtow Cleave", "DARK", "PHYSICAL", 85],
      ["Darkest Lariat", "DARK", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    984: [
      // Great Tusk — GROUND/FIGHTING
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    985: [
      // Scream Tail — FAIRY/PSYCHIC
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Spirit Break", "FAIRY", "PHYSICAL", 65],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Fae Hammer", "FAIRY", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    986: [
      // Brute Bonnet — GRASS/DARK
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    987: [
      // Flutter Mane — GHOST/FAIRY
      ["Astonish", "GHOST", "SPECIAL", 30],
      ["Infernal Parade", "GHOST", "SPECIAL", 45],
      ["Hex", "GHOST", "SPECIAL", 55],
      ["Night Shade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Shadow Ball", "GHOST", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    988: [
      // Slither Wing — BUG/FIGHTING
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    989: [
      // Sandy Shocks — ELECTRIC/GROUND
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Mud Bomb", "GROUND", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600],
      ["Sandsear Storm", "GROUND", "SPECIAL", 600]
    ],
    990: [
      // Iron Treads — GROUND/STEEL
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["High Horsepower", "GROUND", "PHYSICAL", 85],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    991: [
      // Iron Bundle — ICE/WATER
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    992: [
      // Iron Hands — FIGHTING/ELECTRIC
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    993: [
      // Iron Jugulis — DARK/FLYING
      ["Power Trip", "DARK", "SPECIAL", 25],
      ["Fling", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Night Daze", "DARK", "SPECIAL", 75],
      ["Dark Pulse", "DARK", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    994: [
      // Iron Moth — FIRE/POISON
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Sludge Bomb", "POISON", "SPECIAL", 85],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    995: [
      // Iron Thorns — ROCK/ELECTRIC
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Diamond Storm", "ROCK", "PHYSICAL", 85],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    1e3: [
      // Gholdengo — STEEL/GHOST
      ["Bullet Punch", "STEEL", "SPECIAL", 35],
      ["Gear Grind", "STEEL", "SPECIAL", 45],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Shadow Ball", "GHOST", "SPECIAL", 75],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Meteor Mash", "STEEL", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Doom Desire", "STEEL", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    1005: [
      // Roaring Moon — DRAGON/DARK
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 75],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 85],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    1006: [
      // Iron Valiant — FAIRY/FIGHTING
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Fae Hammer", "FAIRY", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    1013: [
      // Sinistcha — GRASS/GHOST
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Matcha Gotcha", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    1018: [
      // Archaludon — STEEL/DRAGON
      ["Metal Claw", "STEEL", "SPECIAL", 35],
      ["Gyro Ball", "STEEL", "SPECIAL", 45],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Flash Cannon", "STEEL", "SPECIAL", 75],
      ["Mirror Shot", "STEEL", "SPECIAL", 80],
      ["Meteor Mash", "STEEL", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600]
    ],
    1019: [
      // Hydrapple — GRASS/DRAGON
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Grass Knot", "GRASS", "SPECIAL", 85],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ]
  };
  var GOLPES_OUTLAND_2026_10_27 = {
    2008: [
      // Charged Raichu — ELECTRIC · base Raichu (#26)
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Spark", "ELECTRIC", "PHYSICAL", 70],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    2013: [
      // Freezing Dewgong — WATER/ICE · base Dewgong (#87)
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    2012: [
      // Evil Cloyster — WATER/ICE · base Cloyster (#91)
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Razor Shell", "WATER", "PHYSICAL", 65],
      ["Ice Spinner", "ICE", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    2055: [
      // Psy Exeggutor — GRASS/PSYCHIC · base Exeggutor (#103)
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Leaf Tornado", "GRASS", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    2039: [
      // Furious Gyarados — WATER/FLYING · base Gyarados (#130)
      ["Surging Strikes", "WATER", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    2045: [
      // Brave Noctowl — NORMAL/FLYING · base Noctowl (#164)
      ["Weather Ball", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Tera Blast", "NORMAL", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mega Onslaught", "NORMAL", "SPECIAL", 600]
    ],
    2036: [
      // Ancient Xatu — PSYCHIC/FLYING · base Xatu (#178)
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    2034: [
      // Brave Steelix — STEEL/GROUND · base Steelix (#208)
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Mega Forge", "STEEL", "PHYSICAL", 600]
    ],
    2043: [
      // Ancient Granbull — FAIRY · base Granbull (#210)
      ["Disarming Voice", "FAIRY", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Draining Kiss", "FAIRY", "PHYSICAL", 50],
      ["Dazzling Gleam", "FAIRY", "PHYSICAL", 65],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Alluring Voice", "FAIRY", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Fleur Cannon", "FAIRY", "PHYSICAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Fae Hammer", "FAIRY", "PHYSICAL", 150],
      ["Let's Snuggle Forever", "FAIRY", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    2053: [
      // Magma Magcargo — FIRE/ROCK · base Magcargo (#219)
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Ancient Power", "ROCK", "SPECIAL", 70],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600]
    ],
    2040: [
      // Brave Mantine — WATER/FLYING · base Mantine (#226)
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    2056: [
      // Banshee Mismagius — GHOST · base Mismagius (#429)
      ["Lick", "GHOST", "SPECIAL", 30],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Hex", "GHOST", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Bitter Malice", "GHOST", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Mega Eclipse", "GHOST", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    2054: [
      // Toxic Toxicroak — FIGHTING/POISON · base Toxicroak (#454)
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Low Kick", "FIGHTING", "PHYSICAL", 45],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600]
    ],
    2052: [
      // Furious Whimsicott — GRASS/FAIRY · base Whimsicott (#547)
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Absorb", "GRASS", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    2057: [
      // Freezing Aurorus — ICE/ROCK · base Aurorus (#699)
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Ancient Power", "ROCK", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Ice Burn", "ICE", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600]
    ]
  };
  var GOLPES_LENDARIOS_2026_10_27 = {
    9: [
      // Blastoise — WATER · não é lendário
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Dive", "WATER", "PHYSICAL", 75],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    257: [
      // Blaziken — FIRE/FIGHTING · não é lendário
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Heat Crash", "FIRE", "PHYSICAL", 60],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 70],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["V-create", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "SPECIAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    489: [
      // Phione — WATER · evolui (→ #490)
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Riptide Slam", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Frost Power", "WATER", "PHYSICAL", 200],
      ["Aqua Crash", "WATER", "PHYSICAL", 600],
      ["Tidal Vortex", "WATER", "SPECIAL", 600]
    ],
    658: [
      // Greninja — WATER/DARK · não é lendário
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Dive", "WATER", "PHYSICAL", 75],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Surf", "WATER", "SPECIAL", 150],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    772: [
      // Type: Null — NORMAL · evolui (→ #773)
      ["Feint", "NORMAL", "PHYSICAL", 30],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Judgment", "NORMAL", "SPECIAL", 100],
      ["Techno Blast", "NORMAL", "SPECIAL", 120]
    ],
    773: [
      // Silvally — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    789: [
      // Cosmog — PSYCHIC · evolui (→ #790)
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120]
    ],
    790: [
      // Cosmoem — PSYCHIC · evolui (→ #791)
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    803: [
      // Poipole — POISON · evolui (→ #804)
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Sludge", "POISON", "SPECIAL", 65],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ]
  };
  var GOLPES_MEGAS_2026_10_27 = {};
  var GOLPES_REWORK_2026_10_29 = {
    4026: [
      // Alolan Raichu — ELECTRIC/PSYCHIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Spark", "ELECTRIC", "PHYSICAL", 70],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    38: [
      // Ninetales — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    53: [
      // Persian — NORMAL
      ["Tail Slap", "NORMAL", "PHYSICAL", 25],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Super Fang", "NORMAL", "PHYSICAL", 50],
      ["Stomp", "NORMAL", "PHYSICAL", 65],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Tera Blast", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Skull Bash", "NORMAL", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    59: [
      // Arcanine — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    73: [
      // Tentacruel — WATER/POISON
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Sludge", "POISON", "SPECIAL", 70],
      ["Liquidation", "WATER", "PHYSICAL", 85],
      ["Dive", "WATER", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    5110: [
      // Galarian Weezing — POISON/FAIRY
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Sludge", "POISON", "SPECIAL", 65],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Toxic Torrent", "POISON", "SPECIAL", 150],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    134: [
      // Vaporeon — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    136: [
      // Flareon — FIRE
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Temper Flare", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Pyro Ball", "FIRE", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    157: [
      // Typhlosion — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Mind Blown", "FIRE", "SPECIAL", 150],
      ["Blast Burn", "FIRE", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    197: [
      // Umbreon — DARK
      ["Dusk Needle", "DARK", "SPECIAL", 30],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Knock Off", "DARK", "PHYSICAL", 65],
      ["Night Daze", "DARK", "SPECIAL", 75],
      ["Dark Pulse", "DARK", "SPECIAL", 80],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    275: [
      // Shiftry — GRASS/DARK
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Dark Pulse", "DARK", "SPECIAL", 75],
      ["Giga Drain", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Solar Blade", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Mega Abyss", "DARK", "SPECIAL", 600]
    ],
    340: [
      // Whiscash — WATER/GROUND
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    389: [
      // Torterra — GRASS/GROUND
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Leaf Blade", "GRASS", "PHYSICAL", 85],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Grove Rampage", "GRASS", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    392: [
      // Infernape — FIRE/FIGHTING
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Heat Crash", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Flamethrower", "FIRE", "SPECIAL", 85],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "SPECIAL", 600]
    ],
    395: [
      // Empoleon — WATER/STEEL
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Flash Cannon", "STEEL", "SPECIAL", 75],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    426: [
      // Drifblim — GHOST/FLYING
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Air Cutter", "FLYING", "SPECIAL", 55],
      ["Hex", "GHOST", "SPECIAL", 65],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Night Shade", "GHOST", "SPECIAL", 85],
      ["Shadow Ball", "GHOST", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    437: [
      // Bronzong — STEEL/PSYCHIC
      ["Tachyon Cutter", "STEEL", "SPECIAL", 35],
      ["Gyro Ball", "STEEL", "PHYSICAL", 45],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Mirror Shot", "STEEL", "SPECIAL", 80],
      ["Flash Cannon", "STEEL", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    455: [
      // Carnivine — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    471: [
      // Glaceon — ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Glaciate", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Ice Beam", "ICE", "SPECIAL", 85],
      ["Freezy Frost", "ICE", "SPECIAL", 90],
      ["Freeze Shock", "ICE", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Glacial Lance", "ICE", "SPECIAL", 600],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    497: [
      // Serperior — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Magical Leaf", "GRASS", "SPECIAL", 60],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Leaf Blade", "GRASS", "PHYSICAL", 85],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Verdant Storm", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    6503: [
      // Hisuian Samurott — WATER/DARK
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    503: [
      // Samurott — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    537: [
      // Seismitoad — WATER/GROUND
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600]
    ],
    631: [
      // Heatmor — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Blast Burn", "FIRE", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    635: [
      // Hydreigon — DARK/DRAGON
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Dragon Breath", "DRAGON", "SPECIAL", 45],
      ["Draco Slime", "DRAGON", "SPECIAL", 55],
      ["Snarl", "DARK", "SPECIAL", 60],
      ["Fickle Beam", "DRAGON", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Draco Deluge", "DRAGON", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600]
    ],
    660: [
      // Diggersby — NORMAL/GROUND
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Scorching Sands", "GROUND", "SPECIAL", 70],
      ["Uproar", "NORMAL", "SPECIAL", 85],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    673: [
      // Gogoat — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Apple Acid", "GRASS", "SPECIAL", 75],
      ["Giga Drain", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    706: [
      // Goodra — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Wyrm Lash", "DRAGON", "PHYSICAL", 70],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Deluge", "DRAGON", "SPECIAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    6706: [
      // Hisuian Goodra — STEEL/DRAGON
      ["Bullet Punch", "STEEL", "PHYSICAL", 35],
      ["Gear Grind", "STEEL", "PHYSICAL", 45],
      ["Gyro Ball", "STEEL", "PHYSICAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Flash Cannon", "STEEL", "SPECIAL", 75],
      ["Mirror Shot", "STEEL", "SPECIAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    724: [
      // Decidueye — GRASS/GHOST
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Grass Pledge", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    727: [
      // Incineroar — FIRE/DARK
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Fire Fang", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Fling", "DARK", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["V-create", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    730: [
      // Primarina — WATER/FAIRY
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Octazooka", "WATER", "SPECIAL", 70],
      ["Moonblast", "FAIRY", "SPECIAL", 85],
      ["Surf", "WATER", "SPECIAL", 90],
      ["Disarming Voice", "FAIRY", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    784: [
      // Kommo-o — DRAGON/FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 40],
      ["Final Gambit", "FIGHTING", "SPECIAL", 55],
      ["Draco Slime", "DRAGON", "SPECIAL", 65],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 70],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Draco Deluge", "DRAGON", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "SPECIAL", 600]
    ],
    815: [
      // Cinderace — FIRE
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Fire Fang", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Pyro Ball", "FIRE", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["V-create", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    871: [
      // Pincurchin — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Double Shock", "ELECTRIC", "PHYSICAL", 120],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    881: [
      // Arctozolt — ELECTRIC/ICE
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Ice Time", "ICE", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    883: [
      // Arctovish — WATER/ICE
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Brine", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    949: [
      // Toedscruel — GROUND/GRASS
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Mud-Slap", "GROUND", "SPECIAL", 40],
      ["Mud Shot", "GROUND", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Seismic Blast", "GROUND", "SPECIAL", 150],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    967: [
      // Cyclizar — DRAGON/NORMAL
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Tera Blast", "NORMAL", "SPECIAL", 75],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    18: [
      // Pidgeot — NORMAL/FLYING
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 35],
      ["Pluck", "FLYING", "PHYSICAL", 45],
      ["Chatter", "FLYING", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Air Slash", "FLYING", "SPECIAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    71: [
      // Victreebel — GRASS/POISON
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Leaf Tornado", "GRASS", "SPECIAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    323: [
      // Camerupt — FIRE/GROUND
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Flame Charge", "FIRE", "PHYSICAL", 60],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Temper Flare", "FIRE", "PHYSICAL", 80],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    448: [
      // Lucario — STEEL/FIGHTING
      ["Vital Throw", "FIGHTING", "PHYSICAL", 30],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 40],
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 50],
      ["Final Gambit", "FIGHTING", "SPECIAL", 60],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 70],
      ["Anchor Shot", "STEEL", "PHYSICAL", 80],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Aura Burst", "FIGHTING", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    604: [
      // Eelektross — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 60],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Mega Thunderstorm", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ]
  };
  var GOLPES_LENDARIOS_2026_10_29 = {
    144: [
      // Articuno — ICE/FLYING
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Ice Hammer", "ICE", "PHYSICAL", 100],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Glacial Lance", "ICE", "SPECIAL", 600],
      ["Ice Time", "ICE", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    5144: [
      // Galarian Articuno — PSYCHIC/FLYING
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600]
    ],
    5145: [
      // Galarian Zapdos — FIGHTING/FLYING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    145: [
      // Zapdos — ELECTRIC/FLYING
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Wildbolt Storm", "ELECTRIC", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Sky Dive", "FLYING", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600],
      ["Volt Surge", "ELECTRIC", "SPECIAL", 600]
    ],
    5146: [
      // Galarian Moltres — DARK/FLYING
      ["Power Trip", "DARK", "SPECIAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Dark Pulse", "DARK", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Night Daze", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    146: [
      // Moltres — FIRE/FLYING
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Ember", "FIRE", "SPECIAL", 50],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Sky Dive", "FLYING", "SPECIAL", 600],
      ["Solar Flare", "FIRE", "SPECIAL", 600]
    ],
    150: [
      // Mewtwo — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Eerie Spell", "PSYCHIC", "SPECIAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Photon Geyser", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Mind Pulse", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    151: [
      // Mew — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 65],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Zen Headbutt", "PSYCHIC", "PHYSICAL", 80],
      ["Freezing Glare", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Mind Pulse", "PSYCHIC", "PHYSICAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    243: [
      // Raikou — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Electro Drift", "ELECTRIC", "SPECIAL", 100],
      ["Electro Shot", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Volt Surge", "ELECTRIC", "SPECIAL", 600],
      ["Voltage Blast", "ELECTRIC", "SPECIAL", 600]
    ],
    244: [
      // Entei — FIRE
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Temper Flare", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Pyro Ball", "FIRE", "PHYSICAL", 120],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Magma Impact", "FIRE", "PHYSICAL", 600],
      ["Solar Flare", "FIRE", "PHYSICAL", 600]
    ],
    245: [
      // Suicune — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Tidal Vortex", "WATER", "SPECIAL", 600]
    ],
    249: [
      // Lugia — PSYCHIC/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Sky Drop", "FLYING", "PHYSICAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Bleakwind Storm", "FLYING", "SPECIAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Mind Pulse", "PSYCHIC", "SPECIAL", 600],
      ["Sky Dive", "FLYING", "SPECIAL", 600]
    ],
    250: [
      // Ho-oh — FIRE/FLYING
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Magma Impact", "FIRE", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    251: [
      // Celebi — PSYCHIC/GRASS
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Heart Stamp", "PSYCHIC", "PHYSICAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Photon Geyser", "PSYCHIC", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Astral Burst", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    377: [
      // Regirock — ROCK
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Geo Pulse", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    378: [
      // Regice — ICE
      ["Ice Ball", "ICE", "SPECIAL", 30],
      ["Powder Snow", "ICE", "SPECIAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Glaciate", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Ice Beam", "ICE", "SPECIAL", 85],
      ["Freezy Frost", "ICE", "SPECIAL", 90],
      ["Blizzard", "ICE", "SPECIAL", 110],
      ["Absolute Zero", "ICE", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Glacial Lance", "ICE", "SPECIAL", 600],
      ["Ice Burn", "ICE", "SPECIAL", 600],
      ["Permafrost", "ICE", "SPECIAL", 600]
    ],
    379: [
      // Registeel — STEEL
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Magnet Bomb", "STEEL", "PHYSICAL", 95],
      ["Sunsteel Strike", "STEEL", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Alloy Surge", "STEEL", "SPECIAL", 600],
      ["Make It Rain", "STEEL", "PHYSICAL", 600]
    ],
    380: [
      // Latias — DRAGON/PSYCHIC
      ["Scale Shot", "DRAGON", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600]
    ],
    381: [
      // Latios — DRAGON/PSYCHIC
      ["Scale Shot", "DRAGON", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600]
    ],
    382: [
      // Kyogre — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Tidal Vortex", "WATER", "SPECIAL", 600]
    ],
    383: [
      // Groudon — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["High Horsepower", "GROUND", "PHYSICAL", 95],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Earthshock", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Sandsear Storm", "GROUND", "PHYSICAL", 600]
    ],
    384: [
      // Rayquaza — DRAGON/FLYING
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Dragon Strike", "DRAGON", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Sky Dive", "FLYING", "SPECIAL", 600]
    ],
    385: [
      // Jirachi — STEEL/PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gyro Ball", "STEEL", "PHYSICAL", 55],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Magnet Bomb", "STEEL", "PHYSICAL", 95],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Alloy Surge", "STEEL", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    483: [
      // Dialga — STEEL/DRAGON · evolui (→ #8483)
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 45],
      ["Gyro Ball", "STEEL", "SPECIAL", 55],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Flash Cannon", "STEEL", "SPECIAL", 90],
      ["Behemoth Bash", "STEEL", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Alloy Surge", "STEEL", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600]
    ],
    8483: [
      // Dialga Origin — STEEL/DRAGON
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 45],
      ["Gyro Ball", "STEEL", "SPECIAL", 55],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Flash Cannon", "STEEL", "SPECIAL", 90],
      ["Behemoth Bash", "STEEL", "SPECIAL", 100],
      ["Doom Desire", "STEEL", "SPECIAL", 120],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600],
      ["Steel Beam", "STEEL", "SPECIAL", 600]
    ],
    484: [
      // Palkia — WATER/DRAGON · evolui (→ #8484)
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Tidal Vortex", "WATER", "SPECIAL", 600]
    ],
    8484: [
      // Palkia Origin — WATER/DRAGON
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600]
    ],
    485: [
      // Heatran — FIRE/STEEL
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 45],
      ["Ember", "FIRE", "SPECIAL", 50],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Mirror Shot", "STEEL", "SPECIAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Heat Wave", "FIRE", "SPECIAL", 95],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Alloy Surge", "STEEL", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Solar Flare", "FIRE", "SPECIAL", 600]
    ],
    486: [
      // Regigigas — NORMAL
      ["Feint", "NORMAL", "PHYSICAL", 30],
      ["Hold Back", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Stomp", "NORMAL", "PHYSICAL", 65],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Endeavor", "NORMAL", "PHYSICAL", 120],
      ["Mega Kick", "NORMAL", "PHYSICAL", 120],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    487: [
      // Giratina — GHOST/DRAGON
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Ominous Wind", "GHOST", "SPECIAL", 60],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Spectral Thief", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 110],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    490: [
      // Manaphy — WATER
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Tidal Vortex", "WATER", "SPECIAL", 600]
    ],
    491: [
      // Darkrai — DARK
      ["Power Trip", "DARK", "SPECIAL", 25],
      ["Fling", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Dark Pulse", "DARK", "SPECIAL", 65],
      ["Lash Out", "DARK", "SPECIAL", 75],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Hyperspace Fury", "DARK", "SPECIAL", 100],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Malicious Moonsault", "DARK", "SPECIAL", 180],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Shadow Burst", "DARK", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    492: [
      // Shaymin — GRASS · evolui (→ #8492)
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 90],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "PHYSICAL", 600],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    8492: [
      // Shaymin Sky — GRASS/FLYING
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600]
    ],
    493: [
      // Arceus — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Judgment", "NORMAL", "SPECIAL", 100],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    494: [
      // Victini — PSYCHIC/FIRE
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Heart Stamp", "PSYCHIC", "PHYSICAL", 55],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 65],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Photon Geyser", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Astral Burst", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Solar Flare", "FIRE", "SPECIAL", 600]
    ],
    638: [
      // Cobalion — STEEL/FIGHTING
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 40],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Double Iron Bash", "STEEL", "PHYSICAL", 60],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 75],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 80],
      ["Flash Cannon", "STEEL", "SPECIAL", 90],
      ["Magnet Bomb", "STEEL", "PHYSICAL", 95],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Alloy Surge", "STEEL", "SPECIAL", 600],
      ["Focus Blast", "FIGHTING", "SPECIAL", 600]
    ],
    639: [
      // Terrakion — ROCK/FIGHTING
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 75],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Geo Pulse", "ROCK", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    640: [
      // Virizion — GRASS/FIGHTING
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 75],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Seed Flare", "GRASS", "SPECIAL", 120],
      ["Flora Burst", "GRASS", "SPECIAL", 600],
      ["Focus Blast", "FIGHTING", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    641: [
      // Tornadus — FLYING · evolui (→ #8641)
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Chatter", "FLYING", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Beak Blast", "FLYING", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Bleakwind Storm", "FLYING", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Sky Dive", "FLYING", "PHYSICAL", 600]
    ],
    8641: [
      // Tornadus Therian — FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Chatter", "FLYING", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Beak Blast", "FLYING", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Bleakwind Storm", "FLYING", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Sky Dive", "FLYING", "PHYSICAL", 600]
    ],
    642: [
      // Thundurus — ELECTRIC/FLYING · evolui (→ #8642)
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fusion Bolt", "ELECTRIC", "PHYSICAL", 100],
      ["Plasma Fists", "ELECTRIC", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    8642: [
      // Thundurus Therian — ELECTRIC/FLYING
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Electro Drift", "ELECTRIC", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    643: [
      // Reshiram — DRAGON/FIRE
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Dragon Darts", "DRAGON", "SPECIAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Torch Song", "FIRE", "SPECIAL", 80],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Dragon Strike", "DRAGON", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Solar Flare", "FIRE", "SPECIAL", 600]
    ],
    644: [
      // Zekrom — DRAGON/ELECTRIC
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Zing Zap", "ELECTRIC", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Dragon Smash", "DRAGON", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    645: [
      // Landorus — GROUND/FLYING · evolui (→ #8645)
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Mud Shot", "GROUND", "SPECIAL", 55],
      ["Mud Bomb", "GROUND", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 80],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    8645: [
      // Landorus Therian — GROUND/FLYING
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bone Club", "GROUND", "PHYSICAL", 65],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    9646: [
      // Black Kyurem — DRAGON/ICE
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    646: [
      // Kyurem — DRAGON/ICE · evolui (→ #8646)
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Powder Snow", "ICE", "SPECIAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Dragon Strike", "DRAGON", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Permafrost", "ICE", "SPECIAL", 600]
    ],
    8646: [
      // White Kyurem — DRAGON/ICE
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Powder Snow", "ICE", "SPECIAL", 40],
      ["Dragon Darts", "DRAGON", "SPECIAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Glacial Lance", "ICE", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600]
    ],
    647: [
      // Keldeo — WATER/FIGHTING · evolui (→ #8647)
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Final Gambit", "FIGHTING", "SPECIAL", 70],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 90],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Focus Blast", "FIGHTING", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Meteor Assault", "FIGHTING", "SPECIAL", 600]
    ],
    8647: [
      // Keldeo Resolute — WATER/FIGHTING
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Final Gambit", "FIGHTING", "SPECIAL", 70],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 90],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Focus Blast", "FIGHTING", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Meteor Assault", "FIGHTING", "SPECIAL", 600]
    ],
    648: [
      // Meloetta — NORMAL/PSYCHIC
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Snore", "NORMAL", "SPECIAL", 50],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Eerie Spell", "PSYCHIC", "SPECIAL", 80],
      ["Revelation Dance", "NORMAL", "SPECIAL", 90],
      ["Swift", "NORMAL", "SPECIAL", 100],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Tera Starstorm", "NORMAL", "SPECIAL", 120],
      ["Blood Moon", "NORMAL", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    649: [
      // Genesect — BUG/STEEL
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Shredder Team", "BUG", "PHYSICAL", 55],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Signal Beam", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Alloy Surge", "STEEL", "SPECIAL", 600],
      ["Bug Impact", "BUG", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    717: [
      // Yveltal — DARK/FLYING
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 80],
      ["Dark Pulse", "DARK", "SPECIAL", 80],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    718: [
      // Zygarde — DRAGON/GROUND · evolui (→ #9718)
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    8718: [
      // Zygarde 10% — DRAGON/GROUND · evolui (→ #718)
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    9718: [
      // Zygarde Complete — DRAGON/GROUND
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Mud Bomb", "GROUND", "SPECIAL", 70],
      ["Scorching Sands", "GROUND", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    719: [
      // Diancie — ROCK/FAIRY
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    720: [
      // Hoopa — PSYCHIC/GHOST · evolui (→ #8720)
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Night Shade", "GHOST", "SPECIAL", 45],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    8720: [
      // Hoopa Unbound — PSYCHIC/DARK
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 65],
      ["Comeuppance", "DARK", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    721: [
      // Volcanion — FIRE/WATER
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Blast Burn", "FIRE", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    785: [
      // Tapu Koko — ELECTRIC/FAIRY
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Supercell Slam", "ELECTRIC", "PHYSICAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    786: [
      // Tapu Lele — PSYCHIC/FAIRY
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    787: [
      // Tapu Bulu — GRASS/FAIRY
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    788: [
      // Tapu Fini — WATER/FAIRY
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600]
    ],
    791: [
      // Solgaleo — PSYCHIC/STEEL
      ["Stored Power", "PSYCHIC", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "PHYSICAL", 90],
      ["Psystrike", "PSYCHIC", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    792: [
      // Lunala — PSYCHIC/GHOST
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Night Shade", "GHOST", "SPECIAL", 45],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    793: [
      // Nihilego — ROCK/POISON
      ["Rock Blast", "ROCK", "SPECIAL", 25],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Tomb", "ROCK", "SPECIAL", 60],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Mighty Cleave", "ROCK", "SPECIAL", 95],
      ["Diamond Storm", "ROCK", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600],
      ["Meteor Beam", "ROCK", "SPECIAL", 600],
      ["Noxious Torque", "POISON", "SPECIAL", 600]
    ],
    794: [
      // Buzzwole — BUG/FIGHTING
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Sky Uppercut", "FIGHTING", "PHYSICAL", 85],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Megahorn", "BUG", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    795: [
      // Pheromosa — BUG/FIGHTING
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 80],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Secret Sword", "FIGHTING", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Megahorn", "BUG", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    796: [
      // Xurkitree — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Wildbolt Storm", "ELECTRIC", "SPECIAL", 100],
      ["Thunder", "ELECTRIC", "SPECIAL", 110],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Plasma Fists", "ELECTRIC", "SPECIAL", 600],
      ["Volt Surge", "ELECTRIC", "SPECIAL", 600]
    ],
    797: [
      // Celesteela — STEEL/FLYING
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Anchor Shot", "STEEL", "PHYSICAL", 80],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Steel Beam", "STEEL", "SPECIAL", 600]
    ],
    798: [
      // Kartana — GRASS/STEEL
      ["Snap Trap", "GRASS", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Smart Strike", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Make It Rain", "STEEL", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Vine Smash", "GRASS", "PHYSICAL", 600]
    ],
    799: [
      // Guzzlord — DARK/DRAGON
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Assurance", "DARK", "PHYSICAL", 60],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Night Daze", "DARK", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    800: [
      // Necrozma — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 600]
    ],
    801: [
      // Magearna — STEEL/FAIRY
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Gear Grind", "STEEL", "SPECIAL", 50],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Behemoth Bash", "STEEL", "SPECIAL", 100],
      ["Doom Desire", "STEEL", "SPECIAL", 120],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Steel Beam", "STEEL", "SPECIAL", 600]
    ],
    802: [
      // Marshadow — FIGHTING/GHOST
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Astral Barrage", "GHOST", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    804: [
      // Naganadel — POISON/DRAGON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Sludge", "POISON", "SPECIAL", 65],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600],
      ["Venom Burst", "POISON", "SPECIAL", 600]
    ],
    805: [
      // Stakataka — ROCK/STEEL
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    806: [
      // Blacephalon — FIRE/GHOST
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Blast Burn", "FIRE", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    807: [
      // Zeraora — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 80],
      ["Discharge", "ELECTRIC", "SPECIAL", 90],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Plasma Fists", "ELECTRIC", "PHYSICAL", 600],
      ["Volt Surge", "ELECTRIC", "SPECIAL", 600]
    ],
    808: [
      // Meltan — STEEL · evolui (→ #809)
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 130],
      ["Alloy Surge", "STEEL", "PHYSICAL", 600],
      ["Make It Rain", "STEEL", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    809: [
      // Melmetal — STEEL
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 130],
      ["Alloy Surge", "STEEL", "PHYSICAL", 600],
      ["Make It Rain", "STEEL", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    888: [
      // Zacian — FAIRY · evolui (→ #8888)
      ["Disarming Voice", "FAIRY", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Draining Kiss", "FAIRY", "PHYSICAL", 50],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Alluring Voice", "FAIRY", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Sparkly Swirl", "FAIRY", "PHYSICAL", 90],
      ["Misty Explosion", "FAIRY", "PHYSICAL", 100],
      ["Springtide Storm", "FAIRY", "PHYSICAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Fleur Cannon", "FAIRY", "PHYSICAL", 600],
      ["Let's Snuggle Forever", "FAIRY", "PHYSICAL", 600],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600]
    ],
    8888: [
      // Zacian Crowned — FAIRY/STEEL
      ["Disarming Voice", "FAIRY", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Draining Kiss", "FAIRY", "PHYSICAL", 50],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Misty Explosion", "FAIRY", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    889: [
      // Zamazenta — FIGHTING · evolui (→ #8889)
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Sky Uppercut", "FIGHTING", "PHYSICAL", 85],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Combat Torque", "FIGHTING", "PHYSICAL", 600],
      ["Focus Blast", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    8889: [
      // Zamazenta Crowned — FIGHTING/STEEL
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    890: [
      // Eternatus — POISON/DRAGON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Noxious Torque", "POISON", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    892: [
      // Urshifu — FIGHTING/DARK
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    893: [
      // Zarude — DARK/GRASS
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    894: [
      // Regieleki — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Electro Drift", "ELECTRIC", "SPECIAL", 100],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Plasma Fists", "ELECTRIC", "PHYSICAL", 600],
      ["Volt Surge", "ELECTRIC", "SPECIAL", 600]
    ],
    895: [
      // Regidrago — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 60],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Dragon Strike", "DRAGON", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    896: [
      // Glastrier — ICE
      ["Triple Axel", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Fang", "ICE", "PHYSICAL", 65],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Ice Hammer", "ICE", "PHYSICAL", 100],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Blizzard", "ICE", "PHYSICAL", 110],
      ["Permafrost Smash", "ICE", "PHYSICAL", 120],
      ["Freeze Shock", "ICE", "PHYSICAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Permafrost", "ICE", "PHYSICAL", 600]
    ],
    897: [
      // Spectrier — GHOST
      ["Shadow Punch", "GHOST", "SPECIAL", 25],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Phantom Force", "GHOST", "SPECIAL", 90],
      ["Spectral Thief", "GHOST", "SPECIAL", 90],
      ["Poltergeist", "GHOST", "SPECIAL", 110],
      ["Phantom Wail", "GHOST", "SPECIAL", 120],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Moongeist Beam", "GHOST", "SPECIAL", 600],
      ["Soul-Stealing 7-Star Strike", "GHOST", "SPECIAL", 600]
    ],
    898: [
      // Calyrex — PSYCHIC/GRASS
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    8905: [
      // Enamorus Therian — FAIRY/FLYING
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    1001: [
      // Wo-Chien — DARK/GRASS
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    1002: [
      // Chien-Pao — DARK/ICE
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Ice Fang", "ICE", "PHYSICAL", 65],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    1003: [
      // Ting-Lu — DARK/GROUND
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Brutal Swing", "DARK", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Darkest Lariat", "DARK", "PHYSICAL", 85],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Land's Wrath", "GROUND", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    1004: [
      // Chi-Yu — DARK/FIRE
      ["Power Trip", "DARK", "SPECIAL", 25],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Dark Pulse", "DARK", "SPECIAL", 65],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Night Daze", "DARK", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Blast Burn", "FIRE", "SPECIAL", 600],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    1007: [
      // Koraidon — FIGHTING/DRAGON
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    1008: [
      // Miraidon — ELECTRIC/DRAGON
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Wildbolt Storm", "ELECTRIC", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    1009: [
      // Walking Wake — WATER/DRAGON
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600]
    ],
    1010: [
      // Iron Leaves — GRASS/PSYCHIC
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Twin Beam", "PSYCHIC", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "PHYSICAL", 90],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    1014: [
      // Okidogi — POISON/FIGHTING
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Sky Uppercut", "FIGHTING", "PHYSICAL", 85],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    1015: [
      // Munkidori — POISON/PSYCHIC
      ["Smog", "POISON", "SPECIAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Noxious Torque", "POISON", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    1016: [
      // Fezandipiti — POISON/FAIRY
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    1017: [
      // Ogerpon — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "PHYSICAL", 600],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    1020: [
      // Gouging Fire — FIRE/DRAGON
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    1021: [
      // Raging Bolt — ELECTRIC/DRAGON
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Wildbolt Storm", "ELECTRIC", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    1022: [
      // Iron Boulder — ROCK/PSYCHIC
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Twin Beam", "PSYCHIC", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "PHYSICAL", 90],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    1023: [
      // Iron Crown — STEEL/PSYCHIC
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Gear Grind", "STEEL", "SPECIAL", 50],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Behemoth Bash", "STEEL", "SPECIAL", 100],
      ["Doom Desire", "STEEL", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Steel Beam", "STEEL", "SPECIAL", 600]
    ],
    1024: [
      // Terapagos — NORMAL
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Judgment", "NORMAL", "SPECIAL", 100],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    1025: [
      // Pecharunt — POISON/GHOST
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Shell Side Arm", "POISON", "SPECIAL", 85],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Astral Barrage", "GHOST", "PHYSICAL", 600],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ]
  };
  var GOLPES_MEGAS_2026_10_29 = {
    9150: [
      // Mega Mewtwo X — PSYCHIC/FIGHTING · 2ª mega do #150
      ["Stored Power", "PSYCHIC", "PHYSICAL", 25],
      ["Power-Up Punch", "FIGHTING", "PHYSICAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Eerie Spell", "PSYCHIC", "PHYSICAL", 80],
      ["Mist Ball", "PSYCHIC", "PHYSICAL", 95],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3150: [
      // Mega Mewtwo Y — PSYCHIC · mega do #150
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Eerie Spell", "PSYCHIC", "SPECIAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Photon Geyser", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Astral Burst", "PSYCHIC", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    3380: [
      // Mega Latias — DRAGON/PSYCHIC · mega do #380
      ["Scale Shot", "DRAGON", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    3381: [
      // Mega Latios — DRAGON/PSYCHIC · mega do #381
      ["Scale Shot", "DRAGON", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Spacial Rend", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    3382: [
      // Primal Kyogre — WATER · mega do #382
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Tidal Vortex", "WATER", "SPECIAL", 600]
    ],
    3383: [
      // Primal Groudon — GROUND/FIRE · mega do #383
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Sand Tomb", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["High Horsepower", "GROUND", "PHYSICAL", 95],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    3384: [
      // Mega Rayquaza — DRAGON/FLYING · mega do #384
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Sky Dive", "FLYING", "PHYSICAL", 600]
    ],
    3485: [
      // Mega Heatran — FIRE/STEEL · mega do #485
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 45],
      ["Ember", "FIRE", "SPECIAL", 50],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Mirror Shot", "STEEL", "SPECIAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Heat Wave", "FIRE", "SPECIAL", 95],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Alloy Surge", "STEEL", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    3491: [
      // Mega Darkrai — DARK · mega do #491
      ["Power Trip", "DARK", "SPECIAL", 25],
      ["Fling", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Dark Pulse", "DARK", "SPECIAL", 65],
      ["Lash Out", "DARK", "SPECIAL", 75],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Hyperspace Fury", "DARK", "SPECIAL", 100],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Malicious Moonsault", "DARK", "SPECIAL", 180],
      ["Dark Impact", "DARK", "SPECIAL", 600],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    3718: [
      // Mega Zygarde — DRAGON/GROUND · mega do #718
      ["Mud-Slap", "GROUND", "SPECIAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Dragon Breath", "DRAGON", "SPECIAL", 55],
      ["Fickle Beam", "DRAGON", "SPECIAL", 65],
      ["Scorching Sands", "GROUND", "SPECIAL", 70],
      ["Mud Bomb", "GROUND", "SPECIAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Land's Wrath", "GROUND", "SPECIAL", 600]
    ],
    3719: [
      // Mega Diancie — ROCK/FAIRY · mega do #719
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    3801: [
      // Mega Magearna — STEEL/FAIRY · mega do #801
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Gear Grind", "STEEL", "SPECIAL", 50],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 50],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Behemoth Bash", "STEEL", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    3807: [
      // Mega Zeraora — ELECTRIC · mega do #807
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 80],
      ["Discharge", "ELECTRIC", "SPECIAL", 90],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Plasma Fists", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    3003: [
      // Mega Venusaur — GRASS/POISON · mega do #3
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Grass Pledge", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Verdant Storm", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    9006: [
      // Mega Charizard X — FIRE/DRAGON · 2ª mega do #6
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 85],
      ["Heat Crash", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Dragon Smash", "DRAGON", "PHYSICAL", 600]
    ],
    3006: [
      // Mega Charizard Y — FIRE/FLYING · mega do #6
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Burst", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Mind Blown", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    3009: [
      // Mega Blastoise — WATER · mega do #9
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Brine", "WATER", "SPECIAL", 65],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Hydro Cannon", "WATER", "SPECIAL", 600]
    ],
    3015: [
      // Mega Beedrill — BUG/POISON · mega do #15
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["U-turn", "BUG", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    3018: [
      // Mega Pidgeot — NORMAL/FLYING · mega do #18
      ["Gust", "FLYING", "SPECIAL", 35],
      ["Air Cutter", "FLYING", "SPECIAL", 45],
      ["Chatter", "FLYING", "SPECIAL", 55],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Tempest Gale", "FLYING", "SPECIAL", 150],
      ["Blood Moon", "NORMAL", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    9026: [
      // Mega Raichu X — ELECTRIC · 2ª mega do #26
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Thunder Shock", "ELECTRIC", "PHYSICAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 85],
      ["Fusion Bolt", "ELECTRIC", "PHYSICAL", 100],
      ["Aura Wheel", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    3026: [
      // Mega Raichu Y — ELECTRIC · mega do #26
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Electro Drift", "ELECTRIC", "SPECIAL", 100],
      ["Electro Shot", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    3036: [
      // Mega Clefable — FAIRY/FLYING · mega do #36
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Chatter", "FLYING", "SPECIAL", 65],
      ["Alluring Voice", "FAIRY", "SPECIAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    3065: [
      // Mega Alakazam — PSYCHIC · mega do #65
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Zen Headbutt", "PSYCHIC", "SPECIAL", 80],
      ["Photon Geyser", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    3071: [
      // Mega Victreebel — GRASS/POISON · mega do #71
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Leaf Tornado", "GRASS", "SPECIAL", 55],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Razor Leaf", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    3080: [
      // Mega Slowbro — WATER/PSYCHIC · mega do #80
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    3094: [
      // Mega Gengar — GHOST/POISON · mega do #94
      ["Shadow Punch", "GHOST", "SPECIAL", 25],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Hex", "GHOST", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Phantom Wail", "GHOST", "SPECIAL", 150],
      ["Toxic Deluge", "POISON", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    3115: [
      // Mega Kangaskhan — NORMAL · mega do #115
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Stomp", "NORMAL", "PHYSICAL", 65],
      ["Crush Claw", "NORMAL", "PHYSICAL", 75],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    3121: [
      // Mega Starmie — WATER/PSYCHIC · mega do #121
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Flip Turn", "WATER", "PHYSICAL", 55],
      ["Dive", "WATER", "PHYSICAL", 65],
      ["Psybeam", "PSYCHIC", "SPECIAL", 70],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    3127: [
      // Mega Pinsir — BUG/FLYING · mega do #127
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["U-turn", "BUG", "PHYSICAL", 70],
      ["X-Scissor", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Megahorn", "BUG", "PHYSICAL", 600],
      ["Sky Attack", "FLYING", "PHYSICAL", 600]
    ],
    3130: [
      // Mega Gyarados — WATER/DARK · mega do #130
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Bite", "DARK", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    3142: [
      // Mega Aerodactyl — ROCK/FLYING · mega do #142
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Throw", "ROCK", "PHYSICAL", 45],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    3149: [
      // Mega Dragonite — DRAGON/FLYING · mega do #149
      ["Scale Shot", "DRAGON", "SPECIAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Dragon Breath", "DRAGON", "SPECIAL", 55],
      ["Fickle Beam", "DRAGON", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Core Enforcer", "DRAGON", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    3154: [
      // Mega Meganium — GRASS/FAIRY · mega do #154
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Matcha Gotcha", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Verdant Storm", "GRASS", "SPECIAL", 150],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    3160: [
      // Mega Feraligatr — WATER/DRAGON · mega do #160
      ["Triple Dive", "WATER", "PHYSICAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    3181: [
      // Mega Ampharos — ELECTRIC/DRAGON · mega do #181
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Wildbolt Storm", "ELECTRIC", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    3208: [
      // Mega Steelix — STEEL/GROUND · mega do #208
      ["Gear Grind", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Smart Strike", "STEEL", "PHYSICAL", 70],
      ["Iron Head", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    3212: [
      // Mega Scizor — BUG/STEEL · mega do #212
      ["Twineedle", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Steamroller", "BUG", "PHYSICAL", 65],
      ["U-turn", "BUG", "PHYSICAL", 70],
      ["X-Scissor", "BUG", "PHYSICAL", 80],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    3214: [
      // Mega Heracross — BUG/FIGHTING · mega do #214
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Pounce", "BUG", "PHYSICAL", 50],
      ["Steamroller", "BUG", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Megahorn", "BUG", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3227: [
      // Mega Skarmory — STEEL/FLYING · mega do #227
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gyro Ball", "STEEL", "PHYSICAL", 55],
      ["Sky Drop", "FLYING", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    3229: [
      // Mega Houndoom — DARK/FIRE · mega do #229
      ["Payback", "DARK", "SPECIAL", 35],
      ["Beat Up", "DARK", "SPECIAL", 45],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Fiery Wrath", "DARK", "SPECIAL", 75],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Dark Pulse", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    3248: [
      // Mega Tyranitar — ROCK/DARK · mega do #248
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Throw", "ROCK", "PHYSICAL", 45],
      ["Rock Tomb", "ROCK", "PHYSICAL", 55],
      ["Bite", "DARK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Stone Edge", "ROCK", "PHYSICAL", 100],
      ["Fling", "DARK", "PHYSICAL", 120],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    3254: [
      // Mega Sceptile — GRASS/DRAGON · mega do #254
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    3257: [
      // Mega Blaziken — FIRE/FIGHTING · mega do #257
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Heat Crash", "FIRE", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["V-create", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    3260: [
      // Mega Swampert — WATER/GROUND · mega do #260
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Yawn", "WATER", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 50],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Liquidation", "WATER", "PHYSICAL", 75],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Frost Power", "WATER", "PHYSICAL", 150],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    3282: [
      // Mega Gardevoir — PSYCHIC/FAIRY · mega do #282
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Lumina Crash", "PSYCHIC", "SPECIAL", 80],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    3302: [
      // Mega Sableye — DARK/GHOST · mega do #302
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Hex", "GHOST", "SPECIAL", 65],
      ["Sucker Punch", "DARK", "PHYSICAL", 70],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Umbral Blast", "DARK", "SPECIAL", 150],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "SPECIAL", 600]
    ],
    3303: [
      // Mega Mawile — STEEL/FAIRY · mega do #303
      ["Gear Grind", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    3306: [
      // Mega Aggron — STEEL · mega do #306
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Iron Head", "STEEL", "PHYSICAL", 65],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    3308: [
      // Mega Medicham — FIGHTING/PSYCHIC · mega do #308
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Counter", "FIGHTING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 55],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 70],
      ["Brick Break", "FIGHTING", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3310: [
      // Mega Manectric — ELECTRIC · mega do #310
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Electro Shot", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    3319: [
      // Mega Sharpedo — WATER/DARK · mega do #319
      ["Clamp", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Fishious Rend", "WATER", "PHYSICAL", 85],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    3323: [
      // Mega Camerupt — FIRE/GROUND · mega do #323
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Mud Shot", "GROUND", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Fiery Dance", "FIRE", "SPECIAL", 80],
      ["Cinder Barrage", "FIRE", "SPECIAL", 100],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    3334: [
      // Mega Altaria — DRAGON/FAIRY · mega do #334
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Breath", "DRAGON", "SPECIAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600]
    ],
    3354: [
      // Mega Banette — GHOST · mega do #354
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Rage Fist", "GHOST", "PHYSICAL", 50],
      ["Last Respects", "GHOST", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Spectral Thief", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Astral Barrage", "GHOST", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    3358: [
      // Mega Chimecho — PSYCHIC/STEEL · mega do #358
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    3359: [
      // Mega Absol — DARK · mega do #359
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Assurance", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["False Surrender", "DARK", "PHYSICAL", 80],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Baddy Bad", "DARK", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Wicked Blow", "DARK", "PHYSICAL", 600]
    ],
    3362: [
      // Mega Glalie — ICE · mega do #362
      ["Icicle Spear", "ICE", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Glaciate", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 150],
      ["Glacial Lance", "ICE", "PHYSICAL", 600],
      ["Ice Burn", "ICE", "SPECIAL", 600]
    ],
    3373: [
      // Mega Salamence — DRAGON/FLYING · mega do #373
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 55],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Fly", "FLYING", "PHYSICAL", 75],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    3376: [
      // Mega Metagross — STEEL/PSYCHIC · mega do #376
      ["Confusion", "PSYCHIC", "PHYSICAL", 35],
      ["Twin Beam", "PSYCHIC", "PHYSICAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Hard Press", "STEEL", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Zen Headbutt", "PSYCHIC", "PHYSICAL", 80],
      ["Psystrike", "PSYCHIC", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    3398: [
      // Mega Staraptor — FIGHTING/FLYING · mega do #398
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Brick Break", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3428: [
      // Mega Lopunny — NORMAL/FIGHTING · mega do #428
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Headbutt", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    3445: [
      // Mega Garchomp — DRAGON/GROUND · mega do #445
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 55],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600]
    ],
    9445: [
      // Mega Garchomp Z — DRAGON · 2ª mega do #445
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Fickle Beam", "DRAGON", "SPECIAL", 65],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 75],
      ["Core Enforcer", "DRAGON", "SPECIAL", 85],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "SPECIAL", 600]
    ],
    3448: [
      // Mega Lucario — FIGHTING/STEEL · mega do #448
      ["Vital Throw", "FIGHTING", "PHYSICAL", 30],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Low Kick", "FIGHTING", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Meteor Mash", "STEEL", "PHYSICAL", 85],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    9448: [
      // Mega Lucario Z — FIGHTING/STEEL · 2ª mega do #448
      ["Arm Thrust", "FIGHTING", "SPECIAL", 25],
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 40],
      ["Final Gambit", "FIGHTING", "SPECIAL", 55],
      ["Mirror Shot", "STEEL", "SPECIAL", 65],
      ["Submission", "FIGHTING", "SPECIAL", 75],
      ["Sky Uppercut", "FIGHTING", "SPECIAL", 85],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 90],
      ["Focus Blast", "FIGHTING", "SPECIAL", 120],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Aura Burst", "FIGHTING", "SPECIAL", 150],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600],
      ["Meteor Assault", "FIGHTING", "SPECIAL", 600]
    ],
    3460: [
      // Mega Abomasnow — GRASS/ICE · mega do #460
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    3475: [
      // Mega Gallade — PSYCHIC/FIGHTING · mega do #475
      ["Stored Power", "PSYCHIC", "PHYSICAL", 25],
      ["Twin Beam", "PSYCHIC", "PHYSICAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Psyshock", "PSYCHIC", "PHYSICAL", 80],
      ["Psychic", "PSYCHIC", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3478: [
      // Mega Froslass — ICE/GHOST · mega do #478
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Ice Burn", "ICE", "SPECIAL", 600]
    ],
    3500: [
      // Mega Emboar — FIRE/FIGHTING · mega do #500
      ["Fire Spin", "FIRE", "PHYSICAL", 30],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Heat Crash", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Temper Flare", "FIRE", "PHYSICAL", 90],
      ["Raging Fury", "FIRE", "PHYSICAL", 120],
      ["Superpower", "FIGHTING", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["V-create", "FIRE", "PHYSICAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3530: [
      // Mega Excadrill — GROUND/STEEL · mega do #530
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    3531: [
      // Mega Audino — NORMAL/FAIRY · mega do #531
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Hidden Power", "NORMAL", "SPECIAL", 45],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Tri Attack", "NORMAL", "SPECIAL", 75],
      ["Uproar", "NORMAL", "SPECIAL", 85],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    3545: [
      // Mega Scolipede — BUG/POISON · mega do #545
      ["Twineedle", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Bug Bite", "BUG", "PHYSICAL", 55],
      ["Steamroller", "BUG", "PHYSICAL", 65],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    3560: [
      // Mega Scrafty — DARK/FIGHTING · mega do #560
      ["Payback", "DARK", "PHYSICAL", 35],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Darkest Lariat", "DARK", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3604: [
      // Mega Eelektross — ELECTRIC · mega do #604
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Volt Breaker", "ELECTRIC", "PHYSICAL", 150],
      ["Bolt Strike", "ELECTRIC", "PHYSICAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    3609: [
      // Mega Chandelure — GHOST/FIRE · mega do #609
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Burning Jealousy", "FIRE", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Fiery Dance", "FIRE", "SPECIAL", 80],
      ["Heat Wave", "FIRE", "SPECIAL", 95],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    3623: [
      // Mega Golurk — GROUND/GHOST · mega do #623
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Tectonic Slam", "GROUND", "PHYSICAL", 150],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    3652: [
      // Mega Chesnaught — GRASS/FIGHTING · mega do #652
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Horn Leech", "GRASS", "PHYSICAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Timber Crash", "GRASS", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    3655: [
      // Mega Delphox — FIRE/PSYCHIC · mega do #655
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Mystical Fire", "FIRE", "SPECIAL", 75],
      ["Psychic", "PSYCHIC", "SPECIAL", 85],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Mind Blown", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    3658: [
      // Mega Greninja — WATER/DARK · mega do #658
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Dive", "WATER", "PHYSICAL", 75],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Liquidation", "WATER", "PHYSICAL", 95],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    3668: [
      // Mega Pyroar — FIRE/NORMAL · mega do #668
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Burning Jealousy", "FIRE", "SPECIAL", 65],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Overheat", "FIRE", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    3670: [
      // Mega Floette — FAIRY · mega do #670
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 65],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    3678: [
      // Mega Meowstic — PSYCHIC · mega do #678
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    3687: [
      // Mega Malamar — DARK/PSYCHIC · mega do #687
      ["Dusk Needle", "DARK", "SPECIAL", 30],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Feint Attack", "DARK", "PHYSICAL", 60],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Dark Pulse", "DARK", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Midnight Ravage", "DARK", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    3689: [
      // Mega Barbaracle — ROCK/FIGHTING · mega do #689
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Diamond Storm", "ROCK", "PHYSICAL", 85],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    3691: [
      // Mega Dragalge — POISON/DRAGON · mega do #691
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid", "POISON", "SPECIAL", 40],
      ["Acid Spray", "POISON", "SPECIAL", 50],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 75],
      ["Sludge Wave", "POISON", "SPECIAL", 85],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    3701: [
      // Mega Hawlucha — FIGHTING/FLYING · mega do #701
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Acrobatics", "FLYING", "PHYSICAL", 70],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["High Jump Kick", "FIGHTING", "PHYSICAL", 120],
      ["Sky Attack", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3740: [
      // Mega Crabominable — FIGHTING/ICE · mega do #740
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Ice Time", "ICE", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3768: [
      // Mega Golisopod — BUG/STEEL · mega do #768
      ["Fury Cutter", "BUG", "PHYSICAL", 35],
      ["Pin Missile", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Steel Wing", "STEEL", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    3780: [
      // Mega Drampa — NORMAL/DRAGON · mega do #780
      ["Terrain Pulse", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Weather Ball", "NORMAL", "SPECIAL", 50],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    3870: [
      // Mega Falinks — FIGHTING · mega do #870
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    3952: [
      // Mega Scovillain — GRASS/FIRE · mega do #952
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Giga Drain", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    3978: [
      // Mega Tatsugiri — DRAGON/WATER · mega do #978
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Dragon Breath", "DRAGON", "SPECIAL", 45],
      ["Dragon Darts", "DRAGON", "SPECIAL", 50],
      ["Fickle Beam", "DRAGON", "SPECIAL", 65],
      ["Snipe Shot", "WATER", "SPECIAL", 75],
      ["Core Enforcer", "DRAGON", "SPECIAL", 85],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 90],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600]
    ],
    3998: [
      // Mega Baxcalibur — DRAGON/ICE · mega do #998
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Icicle Crash", "ICE", "PHYSICAL", 75],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ]
  };
  var GOLPES_ARTE_NOVA_2026_10_30 = {
    874: [
      // Stonjourner — ROCK · sem evolução
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Stone Axe", "ROCK", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Meteor Beam", "ROCK", "PHYSICAL", 600]
    ],
    905: [
      // Enamorus — FAIRY/FLYING · a lista do Enamorus Therian (`GOLPES_LENDARIOS_2026_10_29`)
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Light of Ruin", "FAIRY", "SPECIAL", 600],
      ["Sky Attack", "FLYING", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    8487: [
      // Giratina Origin — GHOST/DRAGON · a lista do Giratina (`GOLPES_LENDARIOS_2026_10_29`)
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Twister", "DRAGON", "SPECIAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Ominous Wind", "GHOST", "SPECIAL", 60],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Spectral Thief", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 110],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Astral Barrage", "GHOST", "SPECIAL", 600],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    8648: [
      // Meloetta Pirouette — NORMAL/FIGHTING · físico (a Aria é especial)
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 75],
      ["Headbutt", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    8898: [
      // Calyrex Ice Rider — PSYCHIC/ICE · físico
      ["Stored Power", "PSYCHIC", "PHYSICAL", 25],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Psychic", "PSYCHIC", "PHYSICAL", 90],
      ["Ice Hammer", "ICE", "PHYSICAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Permafrost Smash", "ICE", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600],
      ["Glacial Lance", "ICE", "PHYSICAL", 600]
    ],
    9898: [
      // Calyrex Shadow Rider — PSYCHIC/GHOST · especial
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Ominous Wind", "GHOST", "SPECIAL", 45],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Astral Barrage", "GHOST", "SPECIAL", 600]
    ],
    9017: [
      // Ogerpon Wellspring — GRASS/WATER
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Liquidation", "WATER", "PHYSICAL", 85],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    8017: [
      // Ogerpon Hearthflame — GRASS/FIRE
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    12017: [
      // Ogerpon Cornerstone — GRASS/ROCK
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ]
  };
  var GOLPES_MEGAS_2026_10_30 = {
    3970: [
      // Mega Glimmora — ROCK/POISON · mega do #970 (a lista do Glimmora)
      ["Rock Blast", "ROCK", "SPECIAL", 25],
      ["Salt Cure", "ROCK", "SPECIAL", 40],
      ["Rock Throw", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Slide", "ROCK", "SPECIAL", 75],
      ["Sludge Wave", "POISON", "SPECIAL", 85],
      ["Diamond Storm", "ROCK", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ]
  };
  var GOLPES_REWORK_2026_11_02 = {
    24: [
      // Arbok — POISON
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    68: [
      // Machamp — FIGHTING
      ["Power-Up Punch", "FIGHTING", "PHYSICAL", 35],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 50],
      ["Storm Throw", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    85: [
      // Dodrio — NORMAL/FLYING
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Pluck", "FLYING", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Endeavor", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    89: [
      // Muk — POISON
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    97: [
      // Hypno — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Psychic Fangs", "PSYCHIC", "PHYSICAL", 600]
    ],
    106: [
      // Hitmonlee — FIGHTING
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Low Kick", "FIGHTING", "PHYSICAL", 45],
      ["Reversal", "FIGHTING", "PHYSICAL", 55],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 65],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 75],
      ["Brick Break", "FIGHTING", "PHYSICAL", 80],
      ["Collision Course", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    107: [
      // Hitmonchan — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 65],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 75],
      ["Low Kick", "FIGHTING", "PHYSICAL", 80],
      ["Brick Break", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    110: [
      // Weezing — POISON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Sludge", "POISON", "SPECIAL", 65],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Dire Claw", "POISON", "PHYSICAL", 80],
      ["Poison Jab", "POISON", "PHYSICAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    119: [
      // Seaking — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Waterfall", "WATER", "PHYSICAL", 75],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Frost Power", "WATER", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Hydro Cannon", "WATER", "PHYSICAL", 600]
    ],
    128: [
      // Tauros — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Horn Attack", "NORMAL", "PHYSICAL", 65],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Raging Bull", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    169: [
      // Crobat — POISON/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Pluck", "FLYING", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Aerial Ace", "FLYING", "PHYSICAL", 120],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    182: [
      // Bellossom — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Grass Knot", "GRASS", "SPECIAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Magical Leaf", "GRASS", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    201: [
      // Unown — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Psychic Fangs", "PSYCHIC", "PHYSICAL", 600]
    ],
    202: [
      // Wobbuffet — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Freezing Glare", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    230: [
      // Kingdra — WATER/DRAGON
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Draconic Soul", "DRAGON", "SPECIAL", 300],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Eternal Dragon", "DRAGON", "SPECIAL", 600]
    ],
    237: [
      // Hitmontop — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Collision Course", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    277: [
      // Swellow — NORMAL/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Pluck", "FLYING", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    297: [
      // Hariyama — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Low Kick", "FIGHTING", "PHYSICAL", 80],
      ["Reversal", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    301: [
      // Delcatty — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Fake Out", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Strength", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    317: [
      // Swalot — POISON
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Sludge", "POISON", "SPECIAL", 65],
      ["Venoshock", "POISON", "SPECIAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    327: [
      // Spinda — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Strength", "NORMAL", "PHYSICAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    332: [
      // Cacturne — GRASS/DARK
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Dark Pulse", "DARK", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Fling", "DARK", "PHYSICAL", 120],
      ["Magical Leaf", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    335: [
      // Zangoose — NORMAL
      ["Quick Attack", "NORMAL", "PHYSICAL", 35],
      ["False Swipe", "NORMAL", "PHYSICAL", 40],
      ["Scratch", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Headbutt", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    336: [
      // Seviper — POISON
      ["Poison Tail", "POISON", "PHYSICAL", 35],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Sludge", "POISON", "SPECIAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    346: [
      // Cradily — ROCK/GRASS
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Tomb", "ROCK", "PHYSICAL", 45],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Energy Ball", "GRASS", "SPECIAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600]
    ],
    350: [
      // Milotic — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Tidal Cascade", "WATER", "SPECIAL", 120],
      ["Water Spout", "WATER", "SPECIAL", 150],
      ["Hydro Cannon", "WATER", "SPECIAL", 600],
      ["Mystic Maelstrom", "WATER", "SPECIAL", 600]
    ],
    357: [
      // Tropius — GRASS/FLYING
      ["Magical Leaf", "GRASS", "SPECIAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Razor Leaf", "GRASS", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    386: [
      // Deoxys — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 55],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 70],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600],
      ["Psycho Boost", "PSYCHIC", "PHYSICAL", 600]
    ],
    8386: [
      // Deoxys Attack — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 55],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 70],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600],
      ["Psycho Boost", "PSYCHIC", "PHYSICAL", 600]
    ],
    9386: [
      // Deoxys Defense — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 55],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 70],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600],
      ["Psycho Boost", "PSYCHIC", "PHYSICAL", 600]
    ],
    11386: [
      // Deoxys Speed — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 55],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 70],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600],
      ["Psycho Boost", "PSYCHIC", "PHYSICAL", 600]
    ],
    400: [
      // Bibarel — NORMAL/WATER
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    407: [
      // Roserade — GRASS/POISON
      ["Magical Leaf", "GRASS", "SPECIAL", 25],
      ["Absorb", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Petal Bloom", "GRASS", "SPECIAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    411: [
      // Bastiodon — ROCK/STEEL
      ["Pebble Burst", "ROCK", "SPECIAL", 35],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Rock Tomb", "ROCK", "PHYSICAL", 70],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Rock Slide", "ROCK", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Meteor Beam", "ROCK", "SPECIAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600]
    ],
    421: [
      // Cherrim — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Seed Flare", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    432: [
      // Purugly — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Slash", "NORMAL", "PHYSICAL", 65],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    441: [
      // Chatot — NORMAL/FLYING
      ["Gust", "FLYING", "SPECIAL", 35],
      ["Air Cutter", "FLYING", "SPECIAL", 45],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Chatter", "FLYING", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Tempest Gale", "FLYING", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    463: [
      // Lickilicky — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    465: [
      // Tangrowth — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    467: [
      // Magmortar — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Burning Jealousy", "FIRE", "SPECIAL", 65],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Fire Blast", "FIRE", "SPECIAL", 120],
      ["Eruption", "FIRE", "SPECIAL", 150],
      ["Blast Burn", "FIRE", "SPECIAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    468: [
      // Togekiss — FAIRY/FLYING
      ["Fairy Wind", "FAIRY", "SPECIAL", 35],
      ["Pixie Dust", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Alluring Voice", "FAIRY", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    469: [
      // Yanmega — BUG/FLYING
      ["Struggle Bug", "BUG", "SPECIAL", 35],
      ["Infestation", "BUG", "SPECIAL", 40],
      ["Silver Wind", "BUG", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Pollen Puff", "BUG", "SPECIAL", 85],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Swarm Beam", "BUG", "SPECIAL", 120],
      ["Swarm Pulse", "BUG", "SPECIAL", 150],
      ["Hive Crush", "BUG", "SPECIAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    474: [
      // Porygon-Z — NORMAL
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Weather Ball", "NORMAL", "SPECIAL", 45],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Swift", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Techno Blast", "NORMAL", "SPECIAL", 120],
      ["Tera Starstorm", "NORMAL", "SPECIAL", 120],
      ["Hyper Beam", "NORMAL", "SPECIAL", 150],
      ["Blood Moon", "NORMAL", "SPECIAL", 600],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    477: [
      // Dusknoir — GHOST
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Rage Fist", "GHOST", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Soul Reaper", "GHOST", "PHYSICAL", 150],
      ["Astral Barrage", "GHOST", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    480: [
      // Uxie — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Mind Pulse", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    481: [
      // Mesprit — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Mind Pulse", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    482: [
      // Azelf — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600],
      ["Mind Pulse", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "PHYSICAL", 600]
    ],
    488: [
      // Cresselia — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 45],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 65],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Zen Headbutt", "PSYCHIC", "PHYSICAL", 80],
      ["Freezing Glare", "PSYCHIC", "SPECIAL", 90],
      ["Dream Eater", "PSYCHIC", "SPECIAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Astral Burst", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600],
      ["Psychic Fangs", "PSYCHIC", "PHYSICAL", 600]
    ],
    505: [
      // Watchog — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Take Down", "NORMAL", "PHYSICAL", 75],
      ["Bide", "NORMAL", "PHYSICAL", 80],
      ["Super Fang", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    512: [
      // Simisage — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Solar Blade", "GRASS", "PHYSICAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    514: [
      // Simisear — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Heat Wave", "FIRE", "SPECIAL", 85],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Fire Blast", "FIRE", "SPECIAL", 120],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Blast Burn", "FIRE", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    526: [
      // Gigalith — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Head Smash", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 600]
    ],
    534: [
      // Conkeldurr — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    538: [
      // Throh — FIGHTING
      ["Vital Throw", "FIGHTING", "PHYSICAL", 35],
      ["Revenge", "FIGHTING", "PHYSICAL", 45],
      ["Storm Throw", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Reversal", "FIGHTING", "PHYSICAL", 120],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    539: [
      // Sawk — FIGHTING
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Power-Up Punch", "FIGHTING", "PHYSICAL", 50],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Body Press", "FIGHTING", "PHYSICAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Counter", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    542: [
      // Leavanny — BUG/GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Razor Leaf", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Hive Crush", "BUG", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    556: [
      // Maractus — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Chloroblast", "GRASS", "SPECIAL", 150],
      ["Frenzy Plant", "GRASS", "SPECIAL", 600],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    558: [
      // Crustle — BUG/ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Stone Edge", "ROCK", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120],
      ["Rock Wrecker", "ROCK", "PHYSICAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    569: [
      // Garbodor — POISON
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Venom Crash", "POISON", "PHYSICAL", 150],
      ["Noxious Torque", "POISON", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    586: [
      // Sawsbuck — NORMAL/GRASS
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    589: [
      // Escavalier — BUG/STEEL
      ["Fury Cutter", "BUG", "PHYSICAL", 35],
      ["Pounce", "BUG", "PHYSICAL", 45],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["X-Scissor", "BUG", "PHYSICAL", 65],
      ["Smart Strike", "STEEL", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Bee Swarm", "BUG", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    591: [
      // Amoonguss — GRASS/POISON
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Leaf Storm", "GRASS", "SPECIAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    598: [
      // Ferrothorn — GRASS/STEEL
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Iron Head", "STEEL", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    601: [
      // Klinklang — STEEL
      ["Bullet Punch", "STEEL", "PHYSICAL", 35],
      ["Metal Claw", "STEEL", "PHYSICAL", 45],
      ["Gyro Ball", "STEEL", "PHYSICAL", 55],
      ["Gear Grind", "STEEL", "PHYSICAL", 60],
      ["Smart Strike", "STEEL", "PHYSICAL", 70],
      ["Anchor Shot", "STEEL", "PHYSICAL", 80],
      ["Iron Tail", "STEEL", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Steel Roller", "STEEL", "PHYSICAL", 120],
      ["Gigaton Hammer", "STEEL", "PHYSICAL", 150],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600],
      ["Steel Beam", "STEEL", "PHYSICAL", 600]
    ],
    612: [
      // Haxorus — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 55],
      ["Wyrm Lash", "DRAGON", "PHYSICAL", 65],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    620: [
      // Mienshao — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Reversal", "FIGHTING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 55],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 80],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    621: [
      // Druddigon — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 55],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Wyrm Lash", "DRAGON", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Roar of Time", "DRAGON", "PHYSICAL", 600]
    ],
    626: [
      // Bouffalant — NORMAL
      ["Fury Attack", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Horn Attack", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    628: [
      // Braviary — NORMAL/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 30],
      ["Peck", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Slash", "NORMAL", "PHYSICAL", 70],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Sky Attack", "FLYING", "PHYSICAL", 150],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    630: [
      // Mandibuzz — DARK/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Pluck", "FLYING", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Comeuppance", "DARK", "PHYSICAL", 80],
      ["Aerial Ace", "FLYING", "PHYSICAL", 96],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Sky Attack", "FLYING", "PHYSICAL", 150],
      ["Endless Hollow", "DARK", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    663: [
      // Talonflame — FIRE/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Skyfall Dive", "FLYING", "PHYSICAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    685: [
      // Slurpuff — FAIRY
      ["Pixie Jab", "FAIRY", "PHYSICAL", 35],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Charm Strike", "FAIRY", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Lunar Nova", "FAIRY", "SPECIAL", 150],
      ["Mega Radiance", "FAIRY", "SPECIAL", 600],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    716: [
      // Xerneas — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Charm Strike", "FAIRY", "PHYSICAL", 50],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Misty Explosion", "FAIRY", "SPECIAL", 100],
      ["Springtide Storm", "FAIRY", "SPECIAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 600],
      ["Let's Snuggle Forever", "FAIRY", "PHYSICAL", 600],
      ["Light of Ruin", "FAIRY", "PHYSICAL", 600]
    ],
    735: [
      // Gumshoos — NORMAL
      ["Tackle", "NORMAL", "PHYSICAL", 35],
      ["Super Fang", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 150],
      ["Blood Moon", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    760: [
      // Bewear — NORMAL/FIGHTING
      ["Bind", "NORMAL", "PHYSICAL", 25],
      ["Flail", "NORMAL", "PHYSICAL", 40],
      ["Tackle", "NORMAL", "PHYSICAL", 50],
      ["Strength", "NORMAL", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Superpower", "FIGHTING", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    763: [
      // Tsareena — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Razor Leaf", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    766: [
      // Passimian — FIGHTING
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Low Kick", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Reversal", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 80],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    774: [
      // Minior — ROCK/FLYING · evolui (→ #8774)
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Ancient Power", "ROCK", "SPECIAL", 70],
      ["Diamond Storm", "ROCK", "PHYSICAL", 85],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    8774: [
      // Minior Core — ROCK/FLYING
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Ancient Power", "ROCK", "SPECIAL", 70],
      ["Diamond Storm", "ROCK", "PHYSICAL", 85],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    839: [
      // Coalossal — ROCK/FIRE
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Ancient Power", "ROCK", "SPECIAL", 70],
      ["Diamond Storm", "ROCK", "PHYSICAL", 85],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Gem Storm", "ROCK", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    887: [
      // Dragapult — DRAGON/GHOST
      ["Scale Shot", "DRAGON", "PHYSICAL", 25],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Outrage", "DRAGON", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Draconic Soul", "DRAGON", "PHYSICAL", 300],
      ["Eternal Dragon", "DRAGON", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    979: [
      // Annihilape — FIGHTING/GHOST
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 50],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 65],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Counter", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Focus Punch", "FIGHTING", "PHYSICAL", 150],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    // As cinco da "Rework Moveset 2.0 02-11", a segunda planilha do dia:
    124: [
      // Jynx — ICE/PSYCHIC
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Blizzard", "ICE", "SPECIAL", 120],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Absolute Zero", "ICE", "SPECIAL", 150],
      ["Ice Time", "ICE", "SPECIAL", 600],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    135: [
      // Jolteon — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 120],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Ion Cannon", "ELECTRIC", "SPECIAL", 150],
      ["Bolt Strike", "ELECTRIC", "SPECIAL", 600],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    196: [
      // Espeon — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Heart Stamp", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Eerie Spell", "PSYCHIC", "SPECIAL", 80],
      ["Mist Ball", "PSYCHIC", "SPECIAL", 95],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Synchronoise", "PSYCHIC", "SPECIAL", 120],
      ["Psycho Boost", "PSYCHIC", "SPECIAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Prismatic Laser", "PSYCHIC", "SPECIAL", 600]
    ],
    470: [
      // Leafeon — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Wood Hammer", "GRASS", "PHYSICAL", 120],
      ["Bramble Crush", "GRASS", "PHYSICAL", 150],
      ["Frenzy Plant", "GRASS", "PHYSICAL", 600],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    937: [
      // Ceruledge — FIRE/GHOST
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Poltergeist", "GHOST", "PHYSICAL", 120],
      ["Blazing Charge", "FIRE", "PHYSICAL", 150],
      ["Ignition Point", "FIRE", "PHYSICAL", 600],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ]
  };
  var GOLPES_OUTLAND_2026_11_02 = {
    2038: [
      // Ancient Hypno — PSYCHIC · base Hypno (#97)
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mind Crusher", "PSYCHIC", "PHYSICAL", 150],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600],
      ["Psychic Fangs", "PSYCHIC", "PHYSICAL", 600]
    ]
  };
  var GOLPES_ARTE_NOVA_2026_11_05 = {
    775: [
      // Komala — NORMAL · sem evolução
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ]
  };
  var GOLPES_PRIMEIRO_ESTAGIO_2026_11_05 = {
    1: [
      // Bulbasaur — GRASS/POISON
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Absorb", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Sludge", "POISON", "SPECIAL", 70],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Solar Beam", "GRASS", "SPECIAL", 120]
    ],
    4: [
      // Charmander — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Overheat", "FIRE", "SPECIAL", 120]
    ],
    7: [
      // Squirtle — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120]
    ],
    10: [
      // Caterpie — BUG
      ["Fury Cutter", "BUG", "PHYSICAL", 35],
      ["Fell Stinger", "BUG", "PHYSICAL", 45],
      ["Bug Bite", "BUG", "PHYSICAL", 55],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["First Impression", "BUG", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120]
    ],
    13: [
      // Weedle — BUG/POISON
      ["Pin Missile", "BUG", "PHYSICAL", 30],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120]
    ],
    16: [
      // Pidgey — NORMAL/FLYING
      ["Quick Attack", "NORMAL", "PHYSICAL", 35],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Tackle", "NORMAL", "PHYSICAL", 50],
      ["Wing Attack", "FLYING", "PHYSICAL", 70],
      ["Raging Bull", "NORMAL", "PHYSICAL", 90],
      ["Floaty Fall", "FLYING", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120]
    ],
    29: [
      // Nidoran Female — POISON
      ["Poison Sting", "POISON", "PHYSICAL", 30],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120]
    ],
    32: [
      // Nidoran Male — POISON
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Dire Claw", "POISON", "PHYSICAL", 90],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120]
    ],
    41: [
      // Zubat — POISON/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120]
    ],
    43: [
      // Oddish — GRASS/POISON
      ["Absorb", "GRASS", "SPECIAL", 30],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Acid Spray", "POISON", "SPECIAL", 50],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120]
    ],
    56: [
      // Mankey — FIGHTING
      ["Rock Smash", "FIGHTING", "PHYSICAL", 35],
      ["Power-Up Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120]
    ],
    60: [
      // Poliwag — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120]
    ],
    63: [
      // Abra — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Eerie Spell", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Dream Eater", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120]
    ],
    66: [
      // Machop — FIGHTING
      ["Power-Up Punch", "FIGHTING", "PHYSICAL", 35],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 50],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 90],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120]
    ],
    69: [
      // Bellsprout — GRASS/POISON
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Absorb", "GRASS", "SPECIAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120]
    ],
    4074: [
      // Alolan Geodude — ROCK/ELECTRIC
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Fusion Bolt", "ELECTRIC", "PHYSICAL", 100],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120]
    ],
    74: [
      // Geodude — ROCK/GROUND
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Smack Down", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 70],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120]
    ],
    81: [
      // Magnemite — ELECTRIC/STEEL
      ["Shock Wave", "ELECTRIC", "SPECIAL", 30],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Chrome Beam", "STEEL", "SPECIAL", 100],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120]
    ],
    92: [
      // Gastly — GHOST/POISON
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Infernal Parade", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Sludge", "POISON", "SPECIAL", 70],
      ["Bitter Malice", "GHOST", "SPECIAL", 90],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120]
    ],
    111: [
      // Rhyhorn — GROUND/ROCK
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bulldoze", "GROUND", "PHYSICAL", 45],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["High Horsepower", "GROUND", "PHYSICAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120]
    ],
    116: [
      // Horsea — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 100],
      ["Origin Pulse", "WATER", "SPECIAL", 120]
    ],
    137: [
      // Porygon — NORMAL
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Weather Ball", "NORMAL", "SPECIAL", 45],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Judgment", "NORMAL", "SPECIAL", 100],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120]
    ],
    147: [
      // Dratini — DRAGON
      ["Dual Chop", "DRAGON", "PHYSICAL", 35],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 45],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 55],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 70],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Outrage", "DRAGON", "PHYSICAL", 120]
    ],
    152: [
      // Chikorita — GRASS
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Leafage", "GRASS", "PHYSICAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Solar Beam", "GRASS", "SPECIAL", 120]
    ],
    155: [
      // Cyndaquil — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120]
    ],
    158: [
      // Totodile — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Liquidation", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120]
    ],
    172: [
      // Pichu — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 30],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 70],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Thunder", "ELECTRIC", "SPECIAL", 100],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120]
    ],
    173: [
      // Cleffa — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 30],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Moonblast", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120]
    ],
    174: [
      // Igglybuff — NORMAL/FAIRY
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 45],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Springtide Storm", "FAIRY", "SPECIAL", 100],
      ["Swift", "NORMAL", "SPECIAL", 120]
    ],
    175: [
      // Togepi — FAIRY
      ["Pixie Dust", "FAIRY", "SPECIAL", 30],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 90],
      ["Moonblast", "FAIRY", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120]
    ],
    179: [
      // Mareep — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 30],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 55],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Electro Drift", "ELECTRIC", "SPECIAL", 100],
      ["Thunder", "ELECTRIC", "SPECIAL", 120]
    ],
    187: [
      // Hoppip — GRASS/FLYING
      ["Bullet Seed", "GRASS", "PHYSICAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Bounce", "FLYING", "PHYSICAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120]
    ],
    216: [
      // Teddiursa — NORMAL
      ["Fury Swipes", "NORMAL", "PHYSICAL", 30],
      ["Scratch", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Slash", "NORMAL", "PHYSICAL", 70],
      ["Raging Bull", "NORMAL", "PHYSICAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Thrash", "NORMAL", "PHYSICAL", 120]
    ],
    220: [
      // Swinub — ICE/GROUND
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Avalanche", "ICE", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 70],
      ["Ice Hammer", "ICE", "PHYSICAL", 95],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120]
    ],
    239: [
      // Elekid — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 30],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Fusion Bolt", "ELECTRIC", "PHYSICAL", 100],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120]
    ],
    240: [
      // Magby — FIRE
      ["Cinder Tap", "FIRE", "PHYSICAL", 30],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Wheel", "FIRE", "PHYSICAL", 55],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Cinder Barrage", "FIRE", "SPECIAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120]
    ],
    246: [
      // Larvitar — ROCK/GROUND
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Smack Down", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120]
    ],
    252: [
      // Treecko — GRASS
      ["Absorb", "GRASS", "SPECIAL", 30],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Leaf Storm", "GRASS", "SPECIAL", 120]
    ],
    255: [
      // Torchic — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Cinder Barrage", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120]
    ],
    258: [
      // Mudkip — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120]
    ],
    5263: [
      // Galarian Zigzagoon — DARK/NORMAL
      ["Power Trip", "DARK", "PHYSICAL", 30],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Baddy Bad", "DARK", "SPECIAL", 120]
    ],
    263: [
      // Zigzagoon — NORMAL
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Headbutt", "NORMAL", "PHYSICAL", 55],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120]
    ],
    265: [
      // Wurmple — BUG
      ["Twineedle", "BUG", "PHYSICAL", 30],
      ["Fell Stinger", "BUG", "PHYSICAL", 45],
      ["Fury Cutter", "BUG", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 72],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["First Impression", "BUG", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120]
    ],
    270: [
      // Lotad — WATER/GRASS
      ["Absorb", "GRASS", "SPECIAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Water Gun", "WATER", "SPECIAL", 50],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120]
    ],
    273: [
      // Seedot — GRASS
      ["Snap Trap", "GRASS", "PHYSICAL", 35],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120]
    ],
    280: [
      // Ralts — PSYCHIC/FAIRY
      ["Stored Power", "PSYCHIC", "SPECIAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Springtide Storm", "FAIRY", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120]
    ],
    287: [
      // Slakoth — NORMAL
      ["Scratch", "NORMAL", "PHYSICAL", 35],
      ["False Swipe", "NORMAL", "PHYSICAL", 40],
      ["Covet", "NORMAL", "PHYSICAL", 55],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120]
    ],
    293: [
      // Whismur — NORMAL
      ["Pound", "NORMAL", "PHYSICAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Stomp", "NORMAL", "PHYSICAL", 70],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Uproar", "NORMAL", "SPECIAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120]
    ],
    298: [
      // Azurill — NORMAL/FAIRY
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Glimmer Crash", "FAIRY", "PHYSICAL", 100],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120]
    ],
    304: [
      // Aron — STEEL/ROCK
      ["Bullet Punch", "STEEL", "PHYSICAL", 35],
      ["Heavy Slam", "STEEL", "PHYSICAL", 40],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Iron Head", "STEEL", "PHYSICAL", 90],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Steel Roller", "STEEL", "PHYSICAL", 120]
    ],
    328: [
      // Trapinch — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 55],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["High Horsepower", "GROUND", "PHYSICAL", 95],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120]
    ],
    355: [
      // Duskull — GHOST
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Poltergeist", "GHOST", "PHYSICAL", 100],
      ["Shadow Force", "GHOST", "PHYSICAL", 120]
    ],
    363: [
      // Spheal — WATER/ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Freezy Frost", "ICE", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Blizzard", "ICE", "SPECIAL", 120]
    ],
    371: [
      // Bagon — DRAGON
      ["Dual Chop", "DRAGON", "PHYSICAL", 35],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 45],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 55],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Outrage", "DRAGON", "PHYSICAL", 120]
    ],
    374: [
      // Beldum — STEEL/PSYCHIC
      ["Gear Grind", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 55],
      ["Anchor Shot", "STEEL", "PHYSICAL", 80],
      ["Magnet Bomb", "STEEL", "PHYSICAL", 95],
      ["Psionic Smash", "PSYCHIC", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120]
    ],
    387: [
      // Turtwig — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 30],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120]
    ],
    390: [
      // Chimchar — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Heat Wave", "FIRE", "SPECIAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120]
    ],
    393: [
      // Piplup — WATER
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120]
    ],
    396: [
      // Starly — NORMAL/FLYING
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Tackle", "NORMAL", "PHYSICAL", 50],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Floaty Fall", "FLYING", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120]
    ],
    403: [
      // Shinx — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 30],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Fusion Bolt", "ELECTRIC", "PHYSICAL", 100],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120]
    ],
    406: [
      // Budew — GRASS/POISON
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Absorb", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Sludge", "POISON", "SPECIAL", 70],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Malignant Chain", "POISON", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120]
    ],
    439: [
      // Mime Jr. — PSYCHIC/FAIRY
      ["Stored Power", "PSYCHIC", "SPECIAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Springtide Storm", "FAIRY", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120]
    ],
    440: [
      // Happiny — NORMAL
      ["Weather Ball", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Swift", "NORMAL", "SPECIAL", 100],
      ["Techno Blast", "NORMAL", "SPECIAL", 120]
    ],
    443: [
      // Gible — DRAGON/GROUND
      ["Dual Chop", "DRAGON", "PHYSICAL", 35],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 45],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120]
    ],
    495: [
      // Snivy — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 30],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Leaf Storm", "GRASS", "SPECIAL", 120]
    ],
    498: [
      // Tepig — FIRE
      ["Cinder Tap", "FIRE", "PHYSICAL", 30],
      ["Flame Charge", "FIRE", "PHYSICAL", 45],
      ["Fire Fang", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 70],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Heat Crash", "FIRE", "PHYSICAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120]
    ],
    501: [
      // Oshawott — WATER
      ["Water Gun", "WATER", "SPECIAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120]
    ],
    506: [
      // Lillipup — NORMAL
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120]
    ],
    519: [
      // Pidove — NORMAL/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120]
    ],
    524: [
      // Roggenrola — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 30],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Stone Edge", "ROCK", "PHYSICAL", 100],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120]
    ],
    532: [
      // Timburr — FIGHTING
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Low Kick", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Hammer Arm", "FIGHTING", "PHYSICAL", 95],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120]
    ],
    535: [
      // Tympole — WATER
      ["Whirlpool", "WATER", "SPECIAL", 30],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Surf", "WATER", "SPECIAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120]
    ],
    540: [
      // Sewaddle — BUG/GRASS
      ["Fury Cutter", "BUG", "PHYSICAL", 35],
      ["Fell Stinger", "BUG", "PHYSICAL", 45],
      ["Pounce", "BUG", "PHYSICAL", 50],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120]
    ],
    543: [
      // Venipede — BUG/POISON
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 95],
      ["First Impression", "BUG", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120]
    ],
    551: [
      // Sandile — GROUND/DARK
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bulldoze", "GROUND", "PHYSICAL", 55],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["High Horsepower", "GROUND", "PHYSICAL", 95],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120]
    ],
    574: [
      // Gothita — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Expanding Force", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Photon Geyser", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120]
    ],
    577: [
      // Solosis — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Photon Geyser", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120]
    ],
    582: [
      // Vanillite — ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Blizzard", "ICE", "SPECIAL", 100],
      ["Sheer Cold", "ICE", "SPECIAL", 120]
    ],
    599: [
      // Klink — STEEL
      ["Bullet Punch", "STEEL", "PHYSICAL", 35],
      ["Gear Grind", "STEEL", "PHYSICAL", 45],
      ["Gyro Ball", "STEEL", "PHYSICAL", 55],
      ["Anchor Shot", "STEEL", "PHYSICAL", 80],
      ["Magnet Bomb", "STEEL", "PHYSICAL", 95],
      ["Iron Tail", "STEEL", "PHYSICAL", 100],
      ["Steel Roller", "STEEL", "PHYSICAL", 120]
    ],
    602: [
      // Tynamo — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 30],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Fusion Bolt", "ELECTRIC", "PHYSICAL", 100],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120]
    ],
    607: [
      // Litwick — GHOST/FIRE
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Infernal Parade", "GHOST", "SPECIAL", 45],
      ["Night Shade", "GHOST", "SPECIAL", 50],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Phantom Pulse", "GHOST", "SPECIAL", 95],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120]
    ],
    610: [
      // Axew — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 30],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Breaking Swipe", "DRAGON", "PHYSICAL", 55],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Outrage", "DRAGON", "PHYSICAL", 120]
    ],
    624: [
      // Pawniard — DARK/STEEL
      ["Power Trip", "DARK", "PHYSICAL", 30],
      ["Assurance", "DARK", "PHYSICAL", 45],
      ["Metal Claw", "STEEL", "PHYSICAL", 50],
      ["Fling", "DARK", "PHYSICAL", 70],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Iron Tail", "STEEL", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120]
    ],
    633: [
      // Deino — DARK/DRAGON
      ["Fling", "DARK", "PHYSICAL", 35],
      ["Bite", "DARK", "PHYSICAL", 45],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120]
    ],
    650: [
      // Chespin — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 30],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120]
    ],
    653: [
      // Fennekin — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120]
    ],
    656: [
      // Froakie — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Snipe Shot", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120]
    ],
    661: [
      // Fletchling — NORMAL/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 30],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120]
    ],
    664: [
      // Scatterbug — BUG
      ["Fury Cutter", "BUG", "PHYSICAL", 35],
      ["Fell Stinger", "BUG", "PHYSICAL", 45],
      ["Pounce", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["First Impression", "BUG", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120]
    ],
    669: [
      // Flabebe — FAIRY
      ["Pixie Dust", "FAIRY", "SPECIAL", 30],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Springtide Storm", "FAIRY", "SPECIAL", 100],
      ["Disarming Voice", "FAIRY", "SPECIAL", 120]
    ],
    679: [
      // Honedge — STEEL/GHOST
      ["Gear Grind", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Shadow Claw", "GHOST", "PHYSICAL", 55],
      ["Gyro Ball", "STEEL", "PHYSICAL", 70],
      ["Magnet Bomb", "STEEL", "PHYSICAL", 95],
      ["Phantom Force", "GHOST", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120]
    ],
    704: [
      // Goomy — DRAGON
      ["Twister", "DRAGON", "SPECIAL", 35],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Breath", "DRAGON", "SPECIAL", 55],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Draco Deluge", "DRAGON", "SPECIAL", 120]
    ],
    722: [
      // Rowlet — GRASS/FLYING
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Absorb", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120]
    ],
    725: [
      // Litten — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Fire Fang", "FIRE", "PHYSICAL", 70],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Heat Wave", "FIRE", "SPECIAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120]
    ],
    728: [
      // Popplio — WATER
      ["Water Gun", "WATER", "SPECIAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 90],
      ["Torrent Volley", "WATER", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120]
    ],
    731: [
      // Pikipek — NORMAL/FLYING
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Raging Bull", "NORMAL", "PHYSICAL", 90],
      ["Floaty Fall", "FLYING", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120]
    ],
    736: [
      // Grubbin — BUG
      ["Infestation", "BUG", "SPECIAL", 30],
      ["Struggle Bug", "BUG", "SPECIAL", 45],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Chitin Ray", "BUG", "SPECIAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120]
    ],
    761: [
      // Bounsweet — GRASS
      ["Magical Leaf", "GRASS", "SPECIAL", 30],
      ["Bullet Seed", "GRASS", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Leaf Blade", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Grass Knot", "GRASS", "SPECIAL", 120]
    ],
    782: [
      // Jangmo-o — DRAGON
      ["Scale Shot", "DRAGON", "PHYSICAL", 30],
      ["Dragon Tail", "DRAGON", "PHYSICAL", 45],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Outrage", "DRAGON", "PHYSICAL", 120]
    ],
    810: [
      // Grookey — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 30],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120]
    ],
    813: [
      // Scorbunny — FIRE
      ["Cinder Tap", "FIRE", "PHYSICAL", 30],
      ["Flame Wheel", "FIRE", "PHYSICAL", 45],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Blazing Torque", "FIRE", "PHYSICAL", 80],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Sacred Fire", "FIRE", "PHYSICAL", 100],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120]
    ],
    821: [
      // Rookidee — FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 30],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Beak Blast", "FLYING", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120]
    ],
    824: [
      // Blipbug — BUG
      ["Infestation", "BUG", "SPECIAL", 30],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Pollen Puff", "BUG", "SPECIAL", 90],
      ["Chitin Ray", "BUG", "SPECIAL", 100],
      ["Bug Buzz", "BUG", "SPECIAL", 120]
    ],
    837: [
      // Rolycoly — ROCK
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Rock Tomb", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 70],
      ["Quartz Beam", "ROCK", "SPECIAL", 90],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Power Gem", "ROCK", "SPECIAL", 120]
    ],
    840: [
      // Applin — GRASS/DRAGON
      ["Bullet Seed", "GRASS", "PHYSICAL", 30],
      ["Grassy Glide", "GRASS", "PHYSICAL", 45],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Fickle Beam", "DRAGON", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Dragon Rush", "DRAGON", "PHYSICAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120]
    ],
    856: [
      // Hatenna — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psystrike", "PSYCHIC", "SPECIAL", 100],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120]
    ],
    859: [
      // Impidimp — DARK/FAIRY
      ["Dusk Needle", "DARK", "SPECIAL", 30],
      ["Gloom Pulse", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Alluring Voice", "FAIRY", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Sparkly Swirl", "FAIRY", "SPECIAL", 100],
      ["Baddy Bad", "DARK", "SPECIAL", 120]
    ],
    885: [
      // Dreepy — DRAGON/GHOST
      ["Scale Shot", "DRAGON", "PHYSICAL", 30],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Phantom Force", "GHOST", "PHYSICAL", 100],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120]
    ],
    906: [
      // Sprigatito — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 30],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120]
    ],
    909: [
      // Fuecoco — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 30],
      ["Ember", "FIRE", "SPECIAL", 45],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Inferno", "FIRE", "SPECIAL", 100],
      ["Armor Cannon", "FIRE", "SPECIAL", 120]
    ],
    921: [
      // Pawmi — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 30],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Zing Zap", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Supercell Slam", "ELECTRIC", "PHYSICAL", 100],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120]
    ],
    928: [
      // Smoliv — GRASS/NORMAL
      ["Absorb", "GRASS", "SPECIAL", 30],
      ["Syrup Bomb", "GRASS", "SPECIAL", 45],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Judgment", "NORMAL", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120]
    ],
    932: [
      // Nacli — ROCK
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Rock Tomb", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Stone Edge", "ROCK", "PHYSICAL", 120]
    ],
    957: [
      // Tinkatink — FAIRY/STEEL
      ["Pixie Jab", "FAIRY", "PHYSICAL", 35],
      ["Sprite Kick", "FAIRY", "PHYSICAL", 40],
      ["Charm Strike", "FAIRY", "PHYSICAL", 50],
      ["Anchor Shot", "STEEL", "PHYSICAL", 80],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Iron Tail", "STEEL", "PHYSICAL", 100],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120]
    ],
    996: [
      // Frigibax — DRAGON/ICE
      ["Scale Shot", "DRAGON", "PHYSICAL", 30],
      ["Dual Chop", "DRAGON", "PHYSICAL", 40],
      ["Dragon Darts", "DRAGON", "PHYSICAL", 50],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Dragon Hammer", "DRAGON", "PHYSICAL", 90],
      ["Ice Hammer", "ICE", "PHYSICAL", 100],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120]
    ]
  };
  var GOLPES_DUAS_FORMAS_2026_11_05 = {
    4019: [
      // Alolan Rattata — DARK/NORMAL
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Assurance", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    19: [
      // Rattata — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Super Fang", "NORMAL", "PHYSICAL", 72],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    21: [
      // Spearow — NORMAL/FLYING
      ["Peck", "FLYING", "PHYSICAL", 35],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    23: [
      // Ekans — POISON
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Dire Claw", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    4027: [
      // Alolan Sandshrew — ICE/STEEL
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    27: [
      // Sandshrew — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bone Club", "GROUND", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    4037: [
      // Alolan Vulpix — ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Sheer Cold", "ICE", "SPECIAL", 120],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    37: [
      // Vulpix — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Burning Jealousy", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Fiery Dance", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    46: [
      // Paras — BUG/GRASS
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    48: [
      // Venonat — BUG/POISON
      ["Pounce", "BUG", "PHYSICAL", 35],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    4050: [
      // Alolan Diglett — GROUND/STEEL
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    50: [
      // Diglett — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    4052: [
      // Alolan Meowth — DARK
      ["Dusk Needle", "DARK", "SPECIAL", 30],
      ["Gloom Pulse", "DARK", "SPECIAL", 40],
      ["Umbral Wave", "DARK", "SPECIAL", 50],
      ["Snarl", "DARK", "SPECIAL", 60],
      ["Dark Pulse", "DARK", "SPECIAL", 75],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    5052: [
      // Galarian Meowth — STEEL
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Gyro Ball", "STEEL", "PHYSICAL", 60],
      ["Smart Strike", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600]
    ],
    52: [
      // Meowth — NORMAL
      ["Feint", "NORMAL", "PHYSICAL", 30],
      ["Fake Out", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    54: [
      // Psyduck — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    58: [
      // Growlithe — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Fang", "FIRE", "PHYSICAL", 70],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    6058: [
      // Hisuian Growlithe — FIRE/ROCK
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flare Jab", "FIRE", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Flame Charge", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    72: [
      // Tentacool — WATER/POISON
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Water Gun", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Shell Side Arm", "POISON", "SPECIAL", 85],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    5077: [
      // Galarian Ponyta — PSYCHIC
      ["Mind Tap", "PSYCHIC", "PHYSICAL", 30],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 45],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Mind Slam", "PSYCHIC", "PHYSICAL", 85],
      ["Psionic Smash", "PSYCHIC", "PHYSICAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    77: [
      // Ponyta — FIRE
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flare Jab", "FIRE", "PHYSICAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    5079: [
      // Galarian Slowpoke — PSYCHIC
      ["Mind Tap", "PSYCHIC", "PHYSICAL", 30],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 45],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 55],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 70],
      ["Mind Slam", "PSYCHIC", "PHYSICAL", 85],
      ["Psionic Smash", "PSYCHIC", "PHYSICAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    79: [
      // Slowpoke — WATER/PSYCHIC
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Zen Headbutt", "PSYCHIC", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    5083: [
      // Galarian Farfetch'd — FIGHTING
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    84: [
      // Doduo — NORMAL/FLYING
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Pluck", "FLYING", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    86: [
      // Seel — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    4088: [
      // Alolan Grimer — POISON/DARK
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    88: [
      // Grimer — POISON
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Dire Claw", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    90: [
      // Shellder — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    95: [
      // Onix — ROCK/GROUND
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Salt Cure", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    96: [
      // Drowzee — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Mind Strike", "PSYCHIC", "PHYSICAL", 45],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 70],
      ["Psyshield Bash", "PSYCHIC", "PHYSICAL", 80],
      ["Mind Slam", "PSYCHIC", "PHYSICAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    98: [
      // Krabby — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    6100: [
      // Hisuian Voltorb — ELECTRIC/GRASS
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    100: [
      // Voltorb — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Electro Ball", "ELECTRIC", "SPECIAL", 96],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    102: [
      // Exeggcute — GRASS/PSYCHIC
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    104: [
      // Cubone — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Sand Tomb", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    108: [
      // Lickitung — NORMAL · o elo para Lickilicky (#463) entra depois da injeção
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Blood Moon", "NORMAL", "SPECIAL", 600]
    ],
    109: [
      // Koffing — POISON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Dire Claw", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    114: [
      // Tangela — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    118: [
      // Goldeen — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    120: [
      // Staryu — WATER
      ["Water Gun", "WATER", "SPECIAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Brine", "WATER", "SPECIAL", 55],
      ["Octazooka", "WATER", "SPECIAL", 65],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    123: [
      // Scyther — BUG/FLYING
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    129: [
      // Magikarp — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    133: [
      // Eevee — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    138: [
      // Omanyte — ROCK/WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Water Pulse", "WATER", "SPECIAL", 70],
      ["Brine", "WATER", "SPECIAL", 80],
      ["Surf", "WATER", "SPECIAL", 90],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    140: [
      // Kabuto — ROCK/WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Liquidation", "WATER", "PHYSICAL", 85],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    161: [
      // Sentret — NORMAL
      ["Fury Swipes", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Slam", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    163: [
      // Hoothoot — NORMAL/FLYING
      ["Terrain Pulse", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Weather Ball", "NORMAL", "SPECIAL", 50],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Air Slash", "FLYING", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    165: [
      // Ledyba — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Pollen Puff", "BUG", "SPECIAL", 85],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    167: [
      // Spinarak — BUG/POISON
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    170: [
      // Chinchou — WATER/ELECTRIC
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    177: [
      // Natu — PSYCHIC/FLYING
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Dream Eater", "PSYCHIC", "SPECIAL", 85],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    190: [
      // Aipom — NORMAL · o elo para Ambipom (#424) entra depois da injeção
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Head Charge", "NORMAL", "PHYSICAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600]
    ],
    191: [
      // Sunkern — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Razor Leaf", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    193: [
      // Yanma — BUG/FLYING · o elo para Yanmega (#469) entra depois da injeção
      ["Struggle Bug", "BUG", "SPECIAL", 35],
      ["Infestation", "BUG", "SPECIAL", 40],
      ["Silver Wind", "BUG", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Pollen Puff", "BUG", "SPECIAL", 85],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    7194: [
      // Paldean Wooper — POISON/GROUND
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    194: [
      // Wooper — WATER/GROUND
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    198: [
      // Murkrow — DARK/FLYING · o elo para Honchkrow (#430) entra depois da injeção
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Gloom Pulse", "DARK", "SPECIAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Assurance", "DARK", "PHYSICAL", 60],
      ["Sucker Punch", "DARK", "PHYSICAL", 70],
      ["Dark Pulse", "DARK", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    200: [
      // Misdreavus — GHOST · o elo para Mismagius (#429) entra depois da injeção
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Grave Whisper", "GHOST", "SPECIAL", 40],
      ["Infernal Parade", "GHOST", "SPECIAL", 55],
      ["Hex", "GHOST", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Night Shade", "GHOST", "SPECIAL", 80],
      ["Phantom Pulse", "GHOST", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Astral Barrage", "GHOST", "SPECIAL", 600]
    ],
    203: [
      // Girafarig — NORMAL/PSYCHIC · o elo para Farigiraf (#981) entra depois da injeção
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    204: [
      // Pineco — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Pounce", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    206: [
      // Dunsparce — NORMAL · o elo para Dudunsparce (#982) entra depois da injeção
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 45],
      ["Weather Ball", "NORMAL", "SPECIAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Endeavor", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600]
    ],
    207: [
      // Gligar — GROUND/FLYING · o elo para Gliscor (#472) entra depois da injeção
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bone Club", "GROUND", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Aerial Ace", "FLYING", "PHYSICAL", 70],
      ["Earthquake", "GROUND", "PHYSICAL", 85],
      ["High Horsepower", "GROUND", "PHYSICAL", 95],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    209: [
      // Snubbull — FAIRY
      ["Pixie Jab", "FAIRY", "PHYSICAL", 35],
      ["Sprite Kick", "FAIRY", "PHYSICAL", 40],
      ["Charm Strike", "FAIRY", "PHYSICAL", 50],
      ["Fae Strike", "FAIRY", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Glimmer Crash", "FAIRY", "PHYSICAL", 85],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    6211: [
      // Hisuian Qwilfish — DARK/POISON
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Beat Up", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    6215: [
      // Hisuian Sneasel — FIGHTING/POISON
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Cross Chop", "FIGHTING", "PHYSICAL", 100],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    215: [
      // Sneasel — DARK/ICE
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Beat Up", "DARK", "PHYSICAL", 45],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Fling", "DARK", "PHYSICAL", 72],
      ["Comeuppance", "DARK", "PHYSICAL", 80],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    218: [
      // Slugma — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Burst", "FIRE", "SPECIAL", 55],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Flamethrower", "FIRE", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    5222: [
      // Galarian Corsola — GHOST
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Grave Whisper", "GHOST", "SPECIAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Phantom Pulse", "GHOST", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    223: [
      // Remoraid — WATER
      ["Bubble", "WATER", "SPECIAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    228: [
      // Houndour — DARK/FIRE
      ["Dusk Needle", "DARK", "SPECIAL", 30],
      ["Gloom Pulse", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Umbral Wave", "DARK", "SPECIAL", 60],
      ["Burning Jealousy", "FIRE", "SPECIAL", 70],
      ["Dark Pulse", "DARK", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    231: [
      // Phanpy — GROUND
      ["Bone Rush", "GROUND", "PHYSICAL", 25],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Bone Club", "GROUND", "PHYSICAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    234: [
      // Stantler — NORMAL · o elo para Wyrdeer (#899) entra depois da injeção
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 45],
      ["Weather Ball", "NORMAL", "SPECIAL", 50],
      ["Stomp", "NORMAL", "PHYSICAL", 65],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120],
      ["Giga Impact", "NORMAL", "PHYSICAL", 600]
    ],
    236: [
      // Tyrogue — FIGHTING
      ["Vacuum Wave", "FIGHTING", "SPECIAL", 35],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Low Kick", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Final Gambit", "FIGHTING", "SPECIAL", 70],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 80],
      ["Collision Course", "FIGHTING", "PHYSICAL", 100],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "SPECIAL", 600]
    ],
    238: [
      // Smoochum — ICE/PSYCHIC
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Frost Breath", "ICE", "SPECIAL", 45],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Dream Eater", "PSYCHIC", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    261: [
      // Poochyena — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Bite", "DARK", "PHYSICAL", 55],
      ["Assurance", "DARK", "PHYSICAL", 60],
      ["Sucker Punch", "DARK", "PHYSICAL", 70],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    276: [
      // Taillow — NORMAL/FLYING
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Drill Peck", "FLYING", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    278: [
      // Wingull — WATER/FLYING
      ["Gust", "FLYING", "SPECIAL", 35],
      ["Air Cutter", "FLYING", "SPECIAL", 45],
      ["Chatter", "FLYING", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    283: [
      // Surskit — BUG/WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Signal Beam", "BUG", "SPECIAL", 75],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    285: [
      // Shroomish — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    290: [
      // Nincada — BUG/GROUND
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bone Club", "GROUND", "PHYSICAL", 65],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    296: [
      // Makuhita — FIGHTING · o elo para Hariyama (#297) entra depois da injeção
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Counter", "FIGHTING", "PHYSICAL", 55],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Low Kick", "FIGHTING", "PHYSICAL", 80],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Assault", "FIGHTING", "PHYSICAL", 600]
    ],
    299: [
      // Nosepass — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Pebble Burst", "ROCK", "SPECIAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Power Gem", "ROCK", "SPECIAL", 80],
      ["Geode Blast", "ROCK", "SPECIAL", 100],
      ["Meteor Beam", "ROCK", "SPECIAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    300: [
      // Skitty — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Fake Out", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    307: [
      // Meditite — FIGHTING/PSYCHIC
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 40],
      ["Final Gambit", "FIGHTING", "SPECIAL", 55],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Aura Sphere", "FIGHTING", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Dynamic Punch", "FIGHTING", "PHYSICAL", 100],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "SPECIAL", 600]
    ],
    309: [
      // Electrike — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Charge Beam", "ELECTRIC", "SPECIAL", 50],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    316: [
      // Gulpin — POISON
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Sludge", "POISON", "SPECIAL", 65],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    318: [
      // Carvanha — WATER/DARK
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Flip Turn", "WATER", "PHYSICAL", 55],
      ["Jet Punch", "WATER", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    320: [
      // Wailmer — WATER
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Brine", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    322: [
      // Numel — FIRE/GROUND
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Cinder Barrage", "FIRE", "SPECIAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    325: [
      // Spoink — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    331: [
      // Cacnea — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Grass Knot", "GRASS", "SPECIAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    333: [
      // Swablu — NORMAL/FLYING
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Aerial Ace", "FLYING", "PHYSICAL", 55],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    339: [
      // Barboach — WATER/GROUND
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    341: [
      // Corphish — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Crabhammer", "WATER", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    343: [
      // Baltoy — GROUND/PSYCHIC
      ["Mud-Slap", "GROUND", "SPECIAL", 25],
      ["Bonemerang", "GROUND", "PHYSICAL", 45],
      ["Mud Shot", "GROUND", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earth Power", "GROUND", "SPECIAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600]
    ],
    345: [
      // Lileep — ROCK/GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    347: [
      // Anorith — ROCK/BUG
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["X-Scissor", "BUG", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    349: [
      // Feebas — WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    353: [
      // Shuppet — GHOST
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Rage Fist", "GHOST", "PHYSICAL", 60],
      ["Shadow Claw", "GHOST", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    360: [
      // Wynaut — PSYCHIC
      ["Mind Tap", "PSYCHIC", "PHYSICAL", 30],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Dream Eater", "PSYCHIC", "SPECIAL", 55],
      ["Heart Stamp", "PSYCHIC", "SPECIAL", 60],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 70],
      ["Psycho Cut", "PSYCHIC", "PHYSICAL", 80],
      ["Mind Slam", "PSYCHIC", "PHYSICAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    361: [
      // Snorunt — ICE
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Frost Breath", "ICE", "SPECIAL", 60],
      ["Ice Fang", "ICE", "PHYSICAL", 70],
      ["Ice Spinner", "ICE", "PHYSICAL", 80],
      ["Ice Hammer", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    366: [
      // Clamperl — WATER
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    399: [
      // Bidoof — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    401: [
      // Kricketot — BUG
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Chitin Ray", "BUG", "SPECIAL", 100],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    408: [
      // Cranidos — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Tomb", "ROCK", "PHYSICAL", 45],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Rock Throw", "ROCK", "PHYSICAL", 60],
      ["Stone Axe", "ROCK", "PHYSICAL", 70],
      ["Rock Slide", "ROCK", "PHYSICAL", 80],
      ["Stone Edge", "ROCK", "PHYSICAL", 100],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    410: [
      // Shieldon — ROCK/STEEL
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Pebble Burst", "ROCK", "SPECIAL", 40],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Quartz Beam", "ROCK", "SPECIAL", 75],
      ["Iron Head", "STEEL", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600]
    ],
    412: [
      // Burmy — BUG
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Silver Wind", "BUG", "SPECIAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Pollen Puff", "BUG", "SPECIAL", 85],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    415: [
      // Combee — BUG/FLYING
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Lunge", "BUG", "PHYSICAL", 80],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    418: [
      // Buizel — WATER
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Water Gun", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Liquidation", "WATER", "PHYSICAL", 85],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    420: [
      // Cherubi — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    422: [
      // Shellos — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Origin Pulse", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    425: [
      // Drifloon — GHOST/FLYING
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Grave Whisper", "GHOST", "SPECIAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Hex", "GHOST", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Phantom Pulse", "GHOST", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    427: [
      // Buneary — NORMAL
      ["Double Hit", "NORMAL", "PHYSICAL", 35],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    431: [
      // Glameow — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Slash", "NORMAL", "PHYSICAL", 65],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    433: [
      // Chingling — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    434: [
      // Stunky — POISON/DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Bite", "DARK", "PHYSICAL", 60],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Sucker Punch", "DARK", "PHYSICAL", 80],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    436: [
      // Bronzor — STEEL/PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Kinetic Rush", "PSYCHIC", "PHYSICAL", 60],
      ["Psyshock", "PSYCHIC", "SPECIAL", 75],
      ["Mind Slam", "PSYCHIC", "PHYSICAL", 85],
      ["Heavy Slam", "STEEL", "PHYSICAL", 100],
      ["Steel Beam", "STEEL", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "PHYSICAL", 600]
    ],
    438: [
      // Bonsly — ROCK
      ["Rollout", "ROCK", "PHYSICAL", 30],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Diamond Storm", "ROCK", "PHYSICAL", 85],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 95],
      ["Stone Edge", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    446: [
      // Munchlax — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Body Slam", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    447: [
      // Riolu — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Rock Smash", "FIGHTING", "PHYSICAL", 40],
      ["Karate Chop", "FIGHTING", "PHYSICAL", 50],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Reversal", "FIGHTING", "PHYSICAL", 70],
      ["Counter", "FIGHTING", "PHYSICAL", 80],
      ["Brick Break", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    449: [
      // Hippopotas — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Fissure", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Dig", "GROUND", "PHYSICAL", 80],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    451: [
      // Skorupi — POISON/BUG
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    453: [
      // Croagunk — POISON/FIGHTING
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Shell Side Arm", "POISON", "SPECIAL", 85],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Secret Sword", "FIGHTING", "SPECIAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    456: [
      // Finneon — WATER
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 50],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Water Pulse", "WATER", "SPECIAL", 70],
      ["Waterfall", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    458: [
      // Mantyke — WATER/FLYING
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Chilling Water", "WATER", "SPECIAL", 55],
      ["Brine", "WATER", "SPECIAL", 65],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    459: [
      // Snover — GRASS/ICE
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Icy Wind", "ICE", "SPECIAL", 55],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Razor Leaf", "GRASS", "PHYSICAL", 70],
      ["Ice Spinner", "ICE", "PHYSICAL", 80],
      ["Blizzard", "ICE", "SPECIAL", 100],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    504: [
      // Patrat — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Super Fang", "NORMAL", "PHYSICAL", 55],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Hyper Fang", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    509: [
      // Purrloin — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Snarl", "DARK", "SPECIAL", 60],
      ["Dark Pulse", "DARK", "SPECIAL", 75],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    511: [
      // Pansage — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Grass Knot", "GRASS", "SPECIAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Vine Whip", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Leaf Storm", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    513: [
      // Pansear — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Incinerate", "FIRE", "SPECIAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Flame Burst", "FIRE", "SPECIAL", 70],
      ["Fire Punch", "FIRE", "PHYSICAL", 80],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Ignition Point", "FIRE", "SPECIAL", 600]
    ],
    515: [
      // Panpour — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Bubble Beam", "WATER", "SPECIAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Waterfall", "WATER", "PHYSICAL", 75],
      ["Scald", "WATER", "SPECIAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    517: [
      // Munna — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    522: [
      // Blitzle — ELECTRIC
      ["Nuzzle", "ELECTRIC", "PHYSICAL", 25],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 45],
      ["Spark", "ELECTRIC", "PHYSICAL", 55],
      ["Thunder Fang", "ELECTRIC", "PHYSICAL", 65],
      ["Thunder Punch", "ELECTRIC", "PHYSICAL", 75],
      ["Bolt Beak", "ELECTRIC", "PHYSICAL", 85],
      ["Fusion Bolt", "ELECTRIC", "PHYSICAL", 100],
      ["Volt Tackle", "ELECTRIC", "PHYSICAL", 120],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    527: [
      // Woobat — PSYCHIC/FLYING
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Air Cutter", "FLYING", "SPECIAL", 60],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Eerie Spell", "PSYCHIC", "SPECIAL", 80],
      ["Freezing Glare", "PSYCHIC", "SPECIAL", 90],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    529: [
      // Drilbur — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    546: [
      // Cottonee — GRASS/FAIRY
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Glitter Beam", "FAIRY", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    548: [
      // Petilil — GRASS
      ["Magical Leaf", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    550: [
      // Basculin — WATER
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    554: [
      // Darumaka — FIRE
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flare Jab", "FIRE", "PHYSICAL", 40],
      ["Fire Fang", "FIRE", "PHYSICAL", 55],
      ["Flame Wheel", "FIRE", "PHYSICAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Blaze Kick", "FIRE", "PHYSICAL", 85],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    5554: [
      // Galarian Darumaka — ICE
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    557: [
      // Dwebble — BUG/ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Smack Down", "ROCK", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    559: [
      // Scraggy — DARK/FIGHTING
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Assurance", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    5562: [
      // Galarian Yamask — GROUND/GHOST
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    562: [
      // Yamask — GHOST
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Grave Whisper", "GHOST", "SPECIAL", 40],
      ["Infernal Parade", "GHOST", "SPECIAL", 55],
      ["Hex", "GHOST", "SPECIAL", 65],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Phantom Pulse", "GHOST", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    564: [
      // Tirtouga — WATER/ROCK
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    566: [
      // Archen — ROCK/FLYING
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Rock Tomb", "ROCK", "PHYSICAL", 45],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Stone Axe", "ROCK", "PHYSICAL", 65],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    568: [
      // Trubbish — POISON
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Fang", "POISON", "PHYSICAL", 45],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    6570: [
      // Hisuian Zorua — NORMAL/GHOST
      ["Terrain Pulse", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Round", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Swift", "NORMAL", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    570: [
      // Zorua — DARK
      ["Dusk Needle", "DARK", "SPECIAL", 30],
      ["Gloom Pulse", "DARK", "SPECIAL", 40],
      ["Snarl", "DARK", "SPECIAL", 55],
      ["Umbral Wave", "DARK", "SPECIAL", 60],
      ["Night Daze", "DARK", "SPECIAL", 75],
      ["Dark Pulse", "DARK", "SPECIAL", 80],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    572: [
      // Minccino — NORMAL
      ["Tail Slap", "NORMAL", "PHYSICAL", 25],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Crush Grip", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Slam", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    580: [
      // Ducklett — WATER/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Gust", "FLYING", "SPECIAL", 40],
      ["Wing Attack", "FLYING", "PHYSICAL", 55],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Air Slash", "FLYING", "SPECIAL", 75],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    585: [
      // Deerling — NORMAL/GRASS
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Trailblaze", "GRASS", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Take Down", "NORMAL", "PHYSICAL", 85],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    588: [
      // Karrablast — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["X-Scissor", "BUG", "PHYSICAL", 80],
      ["Leech Life", "BUG", "PHYSICAL", 90],
      ["Megahorn", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    590: [
      // Foongus — GRASS/POISON
      ["Magical Leaf", "GRASS", "SPECIAL", 25],
      ["Mega Drain", "GRASS", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    592: [
      // Frillish — WATER/GHOST
      ["Whirlpool", "WATER", "SPECIAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Water Gun", "WATER", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    595: [
      // Joltik — BUG/ELECTRIC
      ["Struggle Bug", "BUG", "SPECIAL", 35],
      ["Infestation", "BUG", "SPECIAL", 40],
      ["Silver Wind", "BUG", "SPECIAL", 55],
      ["Signal Beam", "BUG", "SPECIAL", 65],
      ["Electroweb", "ELECTRIC", "SPECIAL", 70],
      ["Pollen Puff", "BUG", "SPECIAL", 85],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    597: [
      // Ferroseed — GRASS/STEEL
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Gyro Ball", "STEEL", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Drum Beating", "GRASS", "PHYSICAL", 80],
      ["Ivy Cudgel", "GRASS", "PHYSICAL", 100],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    605: [
      // Elgyem — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    613: [
      // Cubchoo — ICE
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Ice Spinner", "ICE", "PHYSICAL", 80],
      ["Ice Hammer", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    616: [
      // Shelmet — BUG
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    619: [
      // Mienfoo — FIGHTING
      ["Arm Thrust", "FIGHTING", "PHYSICAL", 25],
      ["Low Kick", "FIGHTING", "PHYSICAL", 40],
      ["Reversal", "FIGHTING", "PHYSICAL", 55],
      ["Force Palm", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Drain Punch", "FIGHTING", "PHYSICAL", 80],
      ["Collision Course", "FIGHTING", "PHYSICAL", 100],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    622: [
      // Golett — GROUND/GHOST
      ["Astonish", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    627: [
      // Rufflet — NORMAL/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 30],
      ["Peck", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Wing Attack", "FLYING", "PHYSICAL", 60],
      ["Slash", "NORMAL", "PHYSICAL", 70],
      ["Bounce", "FLYING", "PHYSICAL", 85],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    629: [
      // Vullaby — DARK/FLYING
      ["Aerial Ace", "FLYING", "PHYSICAL", 25],
      ["Dual Wingbeat", "FLYING", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Pluck", "FLYING", "PHYSICAL", 60],
      ["Drill Peck", "FLYING", "PHYSICAL", 75],
      ["Comeuppance", "DARK", "PHYSICAL", 80],
      ["Floaty Fall", "FLYING", "PHYSICAL", 90],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Massive Hurricane", "FLYING", "PHYSICAL", 600]
    ],
    636: [
      // Larvesta — BUG/FIRE
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Flare Blitz", "FIRE", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    659: [
      // Bunnelby — NORMAL
      ["Echoed Voice", "NORMAL", "SPECIAL", 35],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 45],
      ["Snore", "NORMAL", "SPECIAL", 50],
      ["Super Fang", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Headbutt", "NORMAL", "PHYSICAL", 80],
      ["Flail", "NORMAL", "PHYSICAL", 90],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    667: [
      // Litleo — FIRE/NORMAL
      ["Weather Ball", "NORMAL", "SPECIAL", 35],
      ["Echoed Voice", "NORMAL", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    672: [
      // Skiddo — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    674: [
      // Pancham — FIGHTING
      ["Vital Throw", "FIGHTING", "PHYSICAL", 28],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Storm Throw", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Low Kick", "FIGHTING", "PHYSICAL", 70],
      ["Low Sweep", "FIGHTING", "PHYSICAL", 80],
      ["Brick Break", "FIGHTING", "PHYSICAL", 90],
      ["Superpower", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    677: [
      // Espurr — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Confusion", "PSYCHIC", "SPECIAL", 50],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Psychic Noise", "PSYCHIC", "SPECIAL", 75],
      ["Expanding Force", "PSYCHIC", "SPECIAL", 80],
      ["Psychic", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    682: [
      // Spritzee — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Glitter Beam", "FAIRY", "SPECIAL", 65],
      ["Alluring Voice", "FAIRY", "SPECIAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    684: [
      // Swirlix — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Glitter Beam", "FAIRY", "SPECIAL", 65],
      ["Alluring Voice", "FAIRY", "SPECIAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    686: [
      // Inkay — DARK/PSYCHIC
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Payback", "DARK", "PHYSICAL", 50],
      ["Knock Off", "DARK", "PHYSICAL", 65],
      ["Night Slash", "DARK", "PHYSICAL", 70],
      ["Mind Slam", "PSYCHIC", "PHYSICAL", 85],
      ["Foul Play", "DARK", "PHYSICAL", 95],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    688: [
      // Binacle — ROCK/WATER
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Razor Shell", "WATER", "PHYSICAL", 75],
      ["Rock Slide", "ROCK", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    690: [
      // Skrelp — POISON/WATER
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Bubble", "WATER", "SPECIAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    692: [
      // Clauncher — WATER
      ["Water Gun", "WATER", "SPECIAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Water Pulse", "WATER", "SPECIAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Crabhammer", "WATER", "PHYSICAL", 85],
      ["Muddy Water", "WATER", "SPECIAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    694: [
      // Helioptile — ELECTRIC/NORMAL
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Terrain Pulse", "NORMAL", "SPECIAL", 50],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Volt Switch", "ELECTRIC", "SPECIAL", 70],
      ["Discharge", "ELECTRIC", "SPECIAL", 80],
      ["Thunderbolt", "ELECTRIC", "SPECIAL", 90],
      ["Hyper Roar", "NORMAL", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    696: [
      // Tyrunt — ROCK/DRAGON
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Wyrm Lash", "DRAGON", "PHYSICAL", 70],
      ["Dragon Claw", "DRAGON", "PHYSICAL", 80],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Glaive Rush", "DRAGON", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    698: [
      // Amaura — ROCK/ICE
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Avalanche", "ICE", "PHYSICAL", 55],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Aurora Beam", "ICE", "SPECIAL", 80],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    708: [
      // Phantump — GHOST/GRASS
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Last Respects", "GHOST", "PHYSICAL", 50],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    710: [
      // Pumpkaboo — GHOST/GRASS
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Rage Fist", "GHOST", "PHYSICAL", 50],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Spirit Shackle", "GHOST", "PHYSICAL", 75],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    712: [
      // Bergmite — ICE
      ["Icicle Spear", "ICE", "PHYSICAL", 25],
      ["Ice Ball", "ICE", "PHYSICAL", 40],
      ["Ice Shard", "ICE", "PHYSICAL", 50],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Fang", "ICE", "PHYSICAL", 70],
      ["Ice Spinner", "ICE", "PHYSICAL", 80],
      ["Ice Hammer", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    714: [
      // Noibat — FLYING/DRAGON
      ["Gust", "FLYING", "SPECIAL", 35],
      ["Air Cutter", "FLYING", "SPECIAL", 45],
      ["Chatter", "FLYING", "SPECIAL", 55],
      ["Air Slash", "FLYING", "SPECIAL", 65],
      ["Draco Slime", "DRAGON", "SPECIAL", 70],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 85],
      ["Aeroblast", "FLYING", "SPECIAL", 100],
      ["Draco Deluge", "DRAGON", "SPECIAL", 120],
      ["Massive Hurricane", "FLYING", "SPECIAL", 600]
    ],
    734: [
      // Yungoos — NORMAL
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Super Fang", "NORMAL", "PHYSICAL", 50],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Facade", "NORMAL", "PHYSICAL", 70],
      ["Endeavor", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Double-Edge", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    739: [
      // Crabrawler — FIGHTING
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    742: [
      // Cutiefly — BUG/FAIRY
      ["Infestation", "BUG", "SPECIAL", 25],
      ["Silver Wind", "BUG", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Signal Beam", "BUG", "SPECIAL", 65],
      ["Pollen Puff", "BUG", "SPECIAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Bug Buzz", "BUG", "SPECIAL", 90],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Hive Crush", "BUG", "SPECIAL", 600]
    ],
    744: [
      // Rockruff — ROCK
      ["Rock Blast", "ROCK", "PHYSICAL", 25],
      ["Accelerock", "ROCK", "PHYSICAL", 40],
      ["Rock Throw", "ROCK", "PHYSICAL", 50],
      ["Rock Tomb", "ROCK", "PHYSICAL", 60],
      ["Rock Slide", "ROCK", "PHYSICAL", 75],
      ["Mighty Cleave", "ROCK", "PHYSICAL", 85],
      ["Diamond Storm", "ROCK", "PHYSICAL", 100],
      ["Boulder Rush", "ROCK", "PHYSICAL", 120],
      ["Crumbling Rain", "ROCK", "PHYSICAL", 600]
    ],
    747: [
      // Mareanie — POISON/WATER
      ["Poison Sting", "POISON", "PHYSICAL", 25],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Toxic Ram", "POISON", "PHYSICAL", 100],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    749: [
      // Mudbray — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["High Horsepower", "GROUND", "PHYSICAL", 85],
      ["Earthquake", "GROUND", "PHYSICAL", 100],
      ["Fissure", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    751: [
      // Dewpider — WATER/BUG
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Water Pulse", "WATER", "SPECIAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Bubble Beam", "WATER", "SPECIAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    753: [
      // Fomantis — GRASS
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Petal Blizzard", "GRASS", "PHYSICAL", 90],
      ["Petal Dance", "GRASS", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    755: [
      // Morelull — GRASS/FAIRY
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Syrup Bomb", "GRASS", "SPECIAL", 55],
      ["Glitter Beam", "FAIRY", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    757: [
      // Salandit — POISON/FIRE
      ["Smog", "POISON", "SPECIAL", 30],
      ["Acid Spray", "POISON", "SPECIAL", 40],
      ["Clear Smog", "POISON", "SPECIAL", 50],
      ["Venoshock", "POISON", "SPECIAL", 65],
      ["Sludge", "POISON", "SPECIAL", 70],
      ["Fire Pledge", "FIRE", "SPECIAL", 80],
      ["Sludge Wave", "POISON", "SPECIAL", 95],
      ["Armor Cannon", "FIRE", "SPECIAL", 120],
      ["Toxic Deluge", "POISON", "SPECIAL", 600]
    ],
    759: [
      // Stufful — NORMAL/FIGHTING
      ["Barrage", "NORMAL", "PHYSICAL", 25],
      ["Tackle", "NORMAL", "PHYSICAL", 40],
      ["Flail", "NORMAL", "PHYSICAL", 50],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Axe Kick", "FIGHTING", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    767: [
      // Wimpod — BUG/WATER
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Waterfall", "WATER", "PHYSICAL", 65],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Lunge", "BUG", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    769: [
      // Sandygast — GHOST/GROUND
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Grave Whisper", "GHOST", "SPECIAL", 40],
      ["Mud Shot", "GROUND", "SPECIAL", 55],
      ["Night Shade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Phantom Pulse", "GHOST", "SPECIAL", 100],
      ["Earth Power", "GROUND", "SPECIAL", 120],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    819: [
      // Skwovet — NORMAL
      ["Feint", "NORMAL", "PHYSICAL", 30],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    827: [
      // Nickit — DARK
      ["Dusk Needle", "DARK", "SPECIAL", 30],
      ["Gloom Pulse", "DARK", "SPECIAL", 40],
      ["Umbral Wave", "DARK", "SPECIAL", 50],
      ["Snarl", "DARK", "SPECIAL", 60],
      ["Dark Pulse", "DARK", "SPECIAL", 75],
      ["Night Daze", "DARK", "SPECIAL", 85],
      ["Fiery Wrath", "DARK", "SPECIAL", 90],
      ["Baddy Bad", "DARK", "SPECIAL", 120],
      ["Endless Hollow", "DARK", "SPECIAL", 600]
    ],
    829: [
      // Gossifleur — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    831: [
      // Wooloo — NORMAL
      ["Feint", "NORMAL", "PHYSICAL", 30],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Hidden Power", "NORMAL", "SPECIAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ],
    833: [
      // Chewtle — WATER
      ["Triple Dive", "WATER", "PHYSICAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    835: [
      // Yamper — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Zippy Zap", "ELECTRIC", "PHYSICAL", 50],
      ["Spark", "ELECTRIC", "PHYSICAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Zing Zap", "ELECTRIC", "PHYSICAL", 80],
      ["Wild Charge", "ELECTRIC", "PHYSICAL", 90],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "PHYSICAL", 600]
    ],
    843: [
      // Silicobra — GROUND
      ["Sand Tomb", "GROUND", "PHYSICAL", 35],
      ["Bone Rush", "GROUND", "PHYSICAL", 40],
      ["Bonemerang", "GROUND", "PHYSICAL", 50],
      ["Bulldoze", "GROUND", "PHYSICAL", 60],
      ["Stomping Tantrum", "GROUND", "PHYSICAL", 75],
      ["Drill Run", "GROUND", "PHYSICAL", 80],
      ["Thousand Waves", "GROUND", "PHYSICAL", 90],
      ["Headlong Rush", "GROUND", "PHYSICAL", 120],
      ["Crossing Fissure", "GROUND", "PHYSICAL", 600]
    ],
    846: [
      // Arrokuda — WATER
      ["Triple Dive", "WATER", "PHYSICAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    848: [
      // Toxel — ELECTRIC/POISON
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Sludge Bomb", "POISON", "SPECIAL", 90],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    850: [
      // Sizzlipede — FIRE/BUG
      ["Cinder Tap", "FIRE", "PHYSICAL", 25],
      ["Flare Jab", "FIRE", "PHYSICAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 55],
      ["Fire Fang", "FIRE", "PHYSICAL", 65],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Fire Lash", "FIRE", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    852: [
      // Clobbopus — FIGHTING
      ["Double Kick", "FIGHTING", "PHYSICAL", 30],
      ["Mach Punch", "FIGHTING", "PHYSICAL", 40],
      ["Seismic Toss", "FIGHTING", "PHYSICAL", 55],
      ["Circle Throw", "FIGHTING", "PHYSICAL", 60],
      ["Brick Break", "FIGHTING", "PHYSICAL", 75],
      ["Submission", "FIGHTING", "PHYSICAL", 80],
      ["Sacred Sword", "FIGHTING", "PHYSICAL", 90],
      ["Close Combat", "FIGHTING", "PHYSICAL", 120],
      ["Meteor Fists", "FIGHTING", "PHYSICAL", 600]
    ],
    854: [
      // Sinistea — GHOST
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Grave Whisper", "GHOST", "SPECIAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Phantom Pulse", "GHOST", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    868: [
      // Milcery — FAIRY
      ["Disarming Voice", "FAIRY", "SPECIAL", 25],
      ["Fairy Wind", "FAIRY", "SPECIAL", 40],
      ["Draining Kiss", "FAIRY", "SPECIAL", 50],
      ["Glitter Beam", "FAIRY", "SPECIAL", 65],
      ["Alluring Voice", "FAIRY", "SPECIAL", 75],
      ["Dazzling Gleam", "FAIRY", "SPECIAL", 80],
      ["Moonblast", "FAIRY", "SPECIAL", 95],
      ["Fleur Cannon", "FAIRY", "SPECIAL", 120],
      ["Starlight Charm", "FAIRY", "SPECIAL", 600]
    ],
    872: [
      // Snom — ICE/BUG
      ["Powder Snow", "ICE", "SPECIAL", 35],
      ["Icy Wind", "ICE", "SPECIAL", 45],
      ["Struggle Bug", "BUG", "SPECIAL", 50],
      ["Aurora Beam", "ICE", "SPECIAL", 65],
      ["Freeze-Dry", "ICE", "SPECIAL", 70],
      ["Freezy Frost", "ICE", "SPECIAL", 85],
      ["Ice Beam", "ICE", "SPECIAL", 90],
      ["Bug Buzz", "BUG", "SPECIAL", 120],
      ["Ice Time", "ICE", "SPECIAL", 600]
    ],
    878: [
      // Cufant — STEEL
      ["Metal Claw", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Gear Grind", "STEEL", "PHYSICAL", 50],
      ["Gyro Ball", "STEEL", "PHYSICAL", 60],
      ["Smart Strike", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Meteor Mash", "STEEL", "PHYSICAL", 90],
      ["Heavy Slam", "STEEL", "PHYSICAL", 120],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600]
    ],
    884: [
      // Duraludon — STEEL/DRAGON
      ["Alloy Spark", "STEEL", "SPECIAL", 35],
      ["Tachyon Cutter", "STEEL", "SPECIAL", 45],
      ["Mirror Shot", "STEEL", "SPECIAL", 55],
      ["Draco Slime", "DRAGON", "SPECIAL", 65],
      ["Dragon Pulse", "DRAGON", "SPECIAL", 75],
      ["Flash Cannon", "STEEL", "SPECIAL", 80],
      ["Chrome Beam", "STEEL", "SPECIAL", 100],
      ["Draco Meteor", "DRAGON", "SPECIAL", 120],
      ["Alloy Breaker", "STEEL", "SPECIAL", 600]
    ],
    915: [
      // Lechonk — NORMAL
      ["Feint", "NORMAL", "PHYSICAL", 30],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    917: [
      // Tarountula — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    919: [
      // Nymble — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    924: [
      // Tandemaus — NORMAL
      ["Feint", "NORMAL", "PHYSICAL", 30],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Egg Bomb", "NORMAL", "PHYSICAL", 55],
      ["Bide", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    926: [
      // Fidough — FAIRY
      ["Pixie Jab", "FAIRY", "PHYSICAL", 35],
      ["Sprite Kick", "FAIRY", "PHYSICAL", 40],
      ["Charm Strike", "FAIRY", "PHYSICAL", 50],
      ["Fae Strike", "FAIRY", "PHYSICAL", 60],
      ["Spirit Break", "FAIRY", "PHYSICAL", 75],
      ["Glimmer Crash", "FAIRY", "PHYSICAL", 85],
      ["Play Rough", "FAIRY", "PHYSICAL", 90],
      ["Magical Torque", "FAIRY", "PHYSICAL", 120],
      ["Starlight Charm", "FAIRY", "PHYSICAL", 600]
    ],
    935: [
      // Charcadet — FIRE
      ["Fire Spin", "FIRE", "SPECIAL", 35],
      ["Ember", "FIRE", "SPECIAL", 40],
      ["Flame Charge", "FIRE", "PHYSICAL", 50],
      ["Incinerate", "FIRE", "SPECIAL", 60],
      ["Fire Punch", "FIRE", "PHYSICAL", 75],
      ["Lava Plume", "FIRE", "SPECIAL", 80],
      ["Bitter Blade", "FIRE", "PHYSICAL", 90],
      ["Fire Blast", "FIRE", "SPECIAL", 120],
      ["Ignition Point", "FIRE", "PHYSICAL", 600]
    ],
    938: [
      // Tadbulb — ELECTRIC
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Overdrive", "ELECTRIC", "SPECIAL", 80],
      ["Buzzy Buzz", "ELECTRIC", "SPECIAL", 90],
      ["Zap Cannon", "ELECTRIC", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    940: [
      // Wattrel — ELECTRIC/FLYING
      ["Shock Wave", "ELECTRIC", "SPECIAL", 25],
      ["Thunder Shock", "ELECTRIC", "SPECIAL", 40],
      ["Electroweb", "ELECTRIC", "SPECIAL", 55],
      ["Parabolic Charge", "ELECTRIC", "SPECIAL", 65],
      ["Rising Voltage", "ELECTRIC", "SPECIAL", 70],
      ["Oblivion Wing", "FLYING", "SPECIAL", 80],
      ["Wildbolt Storm", "ELECTRIC", "SPECIAL", 100],
      ["Hurricane", "FLYING", "SPECIAL", 120],
      ["Static Overload", "ELECTRIC", "SPECIAL", 600]
    ],
    942: [
      // Maschiff — DARK
      ["Power Trip", "DARK", "PHYSICAL", 25],
      ["Fling", "DARK", "PHYSICAL", 40],
      ["Beat Up", "DARK", "PHYSICAL", 55],
      ["Assurance", "DARK", "PHYSICAL", 60],
      ["Lash Out", "DARK", "PHYSICAL", 75],
      ["Crunch", "DARK", "PHYSICAL", 80],
      ["Hyperspace Fury", "DARK", "PHYSICAL", 100],
      ["Pursuit", "DARK", "PHYSICAL", 120],
      ["Endless Hollow", "DARK", "PHYSICAL", 600]
    ],
    944: [
      // Shroodle — POISON/NORMAL
      ["Mortal Spin", "POISON", "PHYSICAL", 30],
      ["Poison Tail", "POISON", "PHYSICAL", 45],
      ["Poison Fang", "POISON", "PHYSICAL", 50],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Cross Poison", "POISON", "PHYSICAL", 70],
      ["Poison Jab", "POISON", "PHYSICAL", 80],
      ["Take Down", "NORMAL", "PHYSICAL", 90],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    946: [
      // Bramblin — GRASS/GHOST
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Branch Poke", "GRASS", "PHYSICAL", 40],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Needle Arm", "GRASS", "PHYSICAL", 60],
      ["Flower Trick", "GRASS", "PHYSICAL", 70],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "PHYSICAL", 600]
    ],
    948: [
      // Toedscool — GROUND/GRASS
      ["Mud-Slap", "GROUND", "SPECIAL", 25],
      ["Sand Burst", "GROUND", "SPECIAL", 40],
      ["Mud Shot", "GROUND", "SPECIAL", 55],
      ["Mud Bomb", "GROUND", "SPECIAL", 65],
      ["Scorching Sands", "GROUND", "SPECIAL", 70],
      ["Energy Ball", "GRASS", "SPECIAL", 85],
      ["Earth Power", "GROUND", "SPECIAL", 90],
      ["Solar Beam", "GRASS", "SPECIAL", 120],
      ["Crossing Fissure", "GROUND", "SPECIAL", 600]
    ],
    951: [
      // Capsakid — GRASS
      ["Bullet Seed", "GRASS", "PHYSICAL", 25],
      ["Vine Whip", "GRASS", "PHYSICAL", 45],
      ["Grassy Glide", "GRASS", "PHYSICAL", 55],
      ["Leaf Tornado", "GRASS", "SPECIAL", 65],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Seed Bomb", "GRASS", "PHYSICAL", 80],
      ["Energy Ball", "GRASS", "SPECIAL", 90],
      ["Power Whip", "GRASS", "PHYSICAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ],
    953: [
      // Rellor — BUG
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Revenge", "BUG", "PHYSICAL", 55],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["First Impression", "BUG", "PHYSICAL", 90],
      ["Attack Order", "BUG", "PHYSICAL", 120],
      ["Hive Crush", "BUG", "PHYSICAL", 600]
    ],
    955: [
      // Flittle — PSYCHIC
      ["Stored Power", "PSYCHIC", "SPECIAL", 25],
      ["Twin Beam", "PSYCHIC", "SPECIAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Psybeam", "PSYCHIC", "SPECIAL", 65],
      ["Mystical Power", "PSYCHIC", "SPECIAL", 70],
      ["Psyshock", "PSYCHIC", "SPECIAL", 80],
      ["Glitzy Glow", "PSYCHIC", "SPECIAL", 90],
      ["Future Sight", "PSYCHIC", "SPECIAL", 120],
      ["Mental Singularity", "PSYCHIC", "SPECIAL", 600]
    ],
    960: [
      // Wiglett — WATER
      ["Triple Dive", "WATER", "PHYSICAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Flip Turn", "WATER", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600]
    ],
    963: [
      // Finizen — WATER
      ["Triple Dive", "WATER", "PHYSICAL", 30],
      ["Aqua Jet", "WATER", "PHYSICAL", 45],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Bubble Beam", "WATER", "SPECIAL", 65],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Dive", "WATER", "PHYSICAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ],
    965: [
      // Varoom — STEEL/POISON
      ["Gear Grind", "STEEL", "PHYSICAL", 35],
      ["Bullet Punch", "STEEL", "PHYSICAL", 40],
      ["Poison Tail", "POISON", "PHYSICAL", 50],
      ["Gyro Ball", "STEEL", "PHYSICAL", 60],
      ["Smart Strike", "STEEL", "PHYSICAL", 70],
      ["Metal Burst", "STEEL", "PHYSICAL", 80],
      ["Behemoth Bash", "STEEL", "PHYSICAL", 100],
      ["Gunk Shot", "POISON", "PHYSICAL", 120],
      ["Alloy Breaker", "STEEL", "PHYSICAL", 600]
    ],
    969: [
      // Glimmet — ROCK/POISON
      ["Grit Shot", "ROCK", "SPECIAL", 30],
      ["Pebble Burst", "ROCK", "SPECIAL", 40],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Ancient Power", "ROCK", "SPECIAL", 60],
      ["Sludge", "POISON", "SPECIAL", 70],
      ["Quartz Beam", "ROCK", "SPECIAL", 85],
      ["Geode Blast", "ROCK", "SPECIAL", 100],
      ["Belch", "POISON", "SPECIAL", 120],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600]
    ],
    971: [
      // Greavard — GHOST
      ["Lick", "GHOST", "PHYSICAL", 30],
      ["Shadow Sneak", "GHOST", "PHYSICAL", 40],
      ["Rage Fist", "GHOST", "PHYSICAL", 50],
      ["Shadow Claw", "GHOST", "PHYSICAL", 65],
      ["Spirit Shackle", "GHOST", "PHYSICAL", 75],
      ["Shadow Bone", "GHOST", "PHYSICAL", 85],
      ["Phantom Force", "GHOST", "PHYSICAL", 90],
      ["Shadow Force", "GHOST", "PHYSICAL", 120],
      ["Untold Nightmare", "GHOST", "PHYSICAL", 600]
    ],
    974: [
      // Cetoddle — ICE
      ["Ice Ball", "ICE", "PHYSICAL", 30],
      ["Ice Shard", "ICE", "PHYSICAL", 40],
      ["Ice Fang", "ICE", "PHYSICAL", 55],
      ["Avalanche", "ICE", "PHYSICAL", 60],
      ["Ice Punch", "ICE", "PHYSICAL", 75],
      ["Icicle Crash", "ICE", "PHYSICAL", 85],
      ["Mountain Gale", "ICE", "PHYSICAL", 100],
      ["Freeze Shock", "ICE", "PHYSICAL", 120],
      ["Ice Time", "ICE", "PHYSICAL", 600]
    ],
    999: [
      // Gimmighoul — GHOST
      ["Wisp Flicker", "GHOST", "SPECIAL", 30],
      ["Grave Whisper", "GHOST", "SPECIAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Infernal Parade", "GHOST", "SPECIAL", 60],
      ["Bitter Malice", "GHOST", "SPECIAL", 75],
      ["Shadow Ball", "GHOST", "SPECIAL", 80],
      ["Phantom Pulse", "GHOST", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Untold Nightmare", "GHOST", "SPECIAL", 600]
    ],
    1012: [
      // Poltchageist — GRASS/GHOST
      ["Absorb", "GRASS", "SPECIAL", 25],
      ["Magical Leaf", "GRASS", "SPECIAL", 40],
      ["Night Shade", "GHOST", "SPECIAL", 55],
      ["Syrup Bomb", "GRASS", "SPECIAL", 60],
      ["Giga Drain", "GRASS", "SPECIAL", 75],
      ["Apple Acid", "GRASS", "SPECIAL", 80],
      ["Grass Knot", "GRASS", "SPECIAL", 100],
      ["Moongeist Beam", "GHOST", "SPECIAL", 120],
      ["Petal Bloom", "GRASS", "SPECIAL", 600]
    ]
  };
  var GOLPES_SEM_EVOLUCAO_2026_11_05 = {
    83: [
      // Farfetchd — NORMAL/FLYING
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Pay Day", "NORMAL", "PHYSICAL", 40],
      ["Acrobatics", "FLYING", "PHYSICAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Chip Away", "NORMAL", "PHYSICAL", 70],
      ["Mega Punch", "NORMAL", "PHYSICAL", 80],
      ["Hyper Drill", "NORMAL", "PHYSICAL", 100],
      ["Dragon Ascent", "FLYING", "PHYSICAL", 120],
      ["Thrash", "NORMAL", "PHYSICAL", 120],
      ["Comet Punch", "NORMAL", "PHYSICAL", 150],
      ["Sky Attack", "FLYING", "PHYSICAL", 600],
      ["Ultimate Force", "NORMAL", "PHYSICAL", 600]
    ],
    211: [
      // Qwilfish — WATER/POISON
      ["Brine Cut", "WATER", "PHYSICAL", 35],
      ["Aqua Jet", "WATER", "PHYSICAL", 40],
      ["Yawn", "WATER", "PHYSICAL", 55],
      ["Barb Barrage", "POISON", "PHYSICAL", 60],
      ["Aqua Cutter", "WATER", "PHYSICAL", 70],
      ["Aqua Step", "WATER", "PHYSICAL", 80],
      ["Aqua Tail", "WATER", "PHYSICAL", 90],
      ["Caustic Slam", "POISON", "PHYSICAL", 120],
      ["Wave Crash", "WATER", "PHYSICAL", 120],
      ["Riptide Slam", "WATER", "PHYSICAL", 150],
      ["Cyclonic Tide", "WATER", "PHYSICAL", 600],
      ["Toxic Deluge", "POISON", "PHYSICAL", 600]
    ],
    222: [
      // Corsola — WATER/ROCK
      ["Water Shuriken", "WATER", "SPECIAL", 25],
      ["Water Gun", "WATER", "SPECIAL", 40],
      ["Crystal Spray", "ROCK", "SPECIAL", 50],
      ["Water Pulse", "WATER", "SPECIAL", 60],
      ["Brine", "WATER", "SPECIAL", 70],
      ["Hydro Steam", "WATER", "SPECIAL", 80],
      ["Bouncy Bubble", "WATER", "SPECIAL", 90],
      ["Hydro Pump", "WATER", "SPECIAL", 120],
      ["Power Gem", "ROCK", "SPECIAL", 120],
      ["Tidal Surge", "WATER", "SPECIAL", 150],
      ["Crumbling Rain", "ROCK", "SPECIAL", 600],
      ["Cyclonic Tide", "WATER", "SPECIAL", 600]
    ]
  };
  var GOLPES_OUTLAND_2026_11_05 = {
    2006: [
      // Furious Scyther — BUG/FLYING · base Scyther (#123)
      ["Pin Missile", "BUG", "PHYSICAL", 25],
      ["Fury Cutter", "BUG", "PHYSICAL", 40],
      ["Fell Stinger", "BUG", "PHYSICAL", 50],
      ["Bug Bite", "BUG", "PHYSICAL", 60],
      ["Skitter Smack", "BUG", "PHYSICAL", 70],
      ["Leech Life", "BUG", "PHYSICAL", 80],
      ["Fly", "FLYING", "PHYSICAL", 90],
      ["Brave Bird", "FLYING", "PHYSICAL", 120],
      ["Mega Swarm", "BUG", "PHYSICAL", 600]
    ],
    2007: [
      // Enigmatic Girafarig — NORMAL/PSYCHIC · base Girafarig (#203)
      ["Double Slap", "NORMAL", "PHYSICAL", 25],
      ["Quick Attack", "NORMAL", "PHYSICAL", 40],
      ["Mirror Coat", "PSYCHIC", "SPECIAL", 55],
      ["Covet", "NORMAL", "PHYSICAL", 60],
      ["Relic Song", "NORMAL", "SPECIAL", 75],
      ["Razor Wind", "NORMAL", "SPECIAL", 80],
      ["Hyper Voice", "NORMAL", "SPECIAL", 90],
      ["Psyblade", "PSYCHIC", "PHYSICAL", 120],
      ["Ultimate Force", "NORMAL", "SPECIAL", 600]
    ]
  };
  var TABELAS = [
    GOLPES_INICIAIS,
    GOLPES_PSEUDO_LENDARIOS,
    GOLPES_MENOS_DE_10,
    GOLPES_REWORK_2026_09,
    GOLPES_PRE_EVOLUCOES_KANTO,
    GOLPES_REWORK_2026_09_28,
    GOLPES_REWORK_2026_10_03,
    GOLPES_REWORK_2026_10_04,
    GOLPES_LENDARIOS_2026_10_11,
    GOLPES_REWORK_2026_10_13,
    GOLPES_REWORK_2026_10_15,
    GOLPES_SEGUNDA_EVO_2026_10_15,
    GOLPES_ARTE_NOVA,
    GOLPES_CONTRATO_2026_10_16,
    GOLPES_REWORK_2026_10_23,
    GOLPES_REWORK_2026_10_26,
    GOLPES_REWORK_2026_10_27,
    GOLPES_LENDARIOS_2026_10_27,
    GOLPES_REWORK_2026_10_29,
    GOLPES_LENDARIOS_2026_10_29,
    GOLPES_ARTE_NOVA_2026_10_30,
    GOLPES_REWORK_2026_11_02,
    GOLPES_ARTE_NOVA_2026_11_05,
    GOLPES_PRIMEIRO_ESTAGIO_2026_11_05,
    GOLPES_DUAS_FORMAS_2026_11_05,
    GOLPES_SEM_EVOLUCAO_2026_11_05
  ];
  var TABELAS_FECHADAS = /* @__PURE__ */ new Set([
    GOLPES_SEGUNDA_EVO_2026_10_15,
    GOLPES_CONTRATO_2026_10_16,
    GOLPES_PRIMEIRO_ESTAGIO_2026_11_05,
    GOLPES_DUAS_FORMAS_2026_11_05
  ]);
  function golpeDaLinha([name, type, category, power, marca]) {
    if (marca === MEGA) {
      return { name, type, category, power, cooldownMs: COOLDOWN_MEGA_MS, learnLevel: 1, categoriaFixa: true, cdFixo: true, golpeMega: true };
    }
    return {
      name,
      type,
      category,
      power,
      cooldownMs: cooldownDoPoder(power),
      learnLevel: 1,
      ...power >= 600 ? { categoriaFixa: true } : {}
    };
  }
  function golpesDaPlanilhaDaMega(pokeId) {
    const linhas = GOLPES_MEGAS_2026_10_30[pokeId] ?? GOLPES_MEGAS_2026_10_29[pokeId] ?? GOLPES_MEGAS_2026_10_27[pokeId] ?? GOLPES_MEGAS_2026_10_26[pokeId] ?? GOLPES_MEGAS_2026_10_23[pokeId] ?? GOLPES_MEGAS_2026_10_22[pokeId] ?? GOLPES_MEGAS_2026_10_21[pokeId] ?? GOLPES_MEGAS_CONTRATO_2026_10_16[pokeId] ?? GOLPES_MEGAS_2026_10_03[pokeId];
    return linhas ? linhas.map(golpeDaLinha) : null;
  }
  function golpesDaPlanilhaDoOutland(pokeId) {
    const linhas = GOLPES_OUTLAND_2026_11_05[pokeId] ?? GOLPES_OUTLAND_2026_11_02[pokeId] ?? GOLPES_OUTLAND_2026_10_27[pokeId] ?? GOLPES_OUTLAND_2026_10_15[pokeId] ?? GOLPES_OUTLAND_2026_10_13[pokeId];
    return linhas ? linhas.map(golpeDaLinha) : null;
  }
  function golpesDoOutlandFechados(pokeId) {
    return Object.hasOwn(GOLPES_OUTLAND_2026_10_15, pokeId) || Object.hasOwn(GOLPES_OUTLAND_2026_11_05, pokeId);
  }
  function aplicarGolpesDasPlanilhas(obter) {
    for (const tabela of TABELAS) {
      const fechada = TABELAS_FECHADAS.has(tabela);
      for (const [id, linhas] of Object.entries(tabela)) {
        for (const esp of obter(Number(id)) ?? []) {
          esp.attacks = linhas.map(golpeDaLinha);
          if (fechada) esp.golpesFechados = true;
          else delete esp.golpesFechados;
        }
      }
    }
  }

  // tests/.cache/shared/herdar-base-outland.mjs
  var herdar_base_outland_exports = {};
  __export(herdar_base_outland_exports, {
    herdarBaseOutland: () => herdarBaseOutland
  });

  // tests/.cache/shared/outland-sprite-dex.mjs
  var PREFIXOS_OUTLAND = [
    "Brave ",
    "Tribal ",
    "Ancient ",
    "War ",
    "Furious ",
    "Charged ",
    "Magnetic ",
    "Evil ",
    "Freezing ",
    "Psy ",
    "Heavy ",
    "Milch-",
    "Roll ",
    "Hard ",
    "Brute ",
    "Enraged ",
    "Dark ",
    "Trickmaster ",
    "Banshee ",
    "Taekwondo ",
    "Enigmatic ",
    "Magma ",
    "Toxic "
  ];
  var DEX_POR_NOME = /* @__PURE__ */ new Map([
    ["blastoise", 9],
    ["feraligatr", 160],
    ["meganium", 154],
    ["venusaur", 3],
    ["heracross", 214],
    ["scyther", 123],
    ["girafarig", 203],
    ["raichu", 26],
    ["ampharos", 181],
    ["electabuzz", 125],
    ["dragonair", 148],
    ["cloyster", 91],
    ["dewgong", 87],
    ["jynx", 124],
    ["piloswine", 221],
    ["miltank", 241],
    ["donphan", 232],
    ["sandslash", 28],
    ["golem", 76],
    ["rhydon", 112],
    ["charizard", 6],
    ["typhlosion", 157],
    ["nidoking", 34],
    ["nidoqueen", 31],
    ["crobat", 169],
    ["gengar", 94],
    ["misdreavus", 200],
    ["hitmonchan", 107],
    ["hitmonlee", 106],
    ["hitmontop", 237],
    ["arcanine", 59],
    ["magmar", 126],
    ["pupitar", 247],
    ["steelix", 208],
    ["wigglytuff", 40],
    ["xatu", 178],
    ["alakazam", 65],
    ["hypno", 97],
    ["gyarados", 130],
    ["mantine", 226],
    ["pinsir", 127],
    ["clefable", 36],
    ["granbull", 210],
    ["skarmory", 227],
    ["noctowl", 164],
    ["pidgeot", 18],
    ["marowak", 105],
    ["tyranitar", 248],
    ["aerodactyl", 142],
    ["scizor", 212],
    ["dragonite", 149],
    ["whimsicott", 547],
    ["magcargo", 219],
    ["toxicroak", 454],
    ["exeggutor", 103],
    ["mismagius", 429],
    ["aurorus", 699],
    ["scrafty", 560]
  ]);
  function dexSpriteBaseOutland(pokeId, nome = "") {
    if (!isOutlandPokeId(pokeId)) return null;
    let base = String(nome ?? "").trim();
    for (const p of PREFIXOS_OUTLAND) {
      if (base.startsWith(p)) {
        base = base.slice(p.length);
        break;
      }
    }
    return DEX_POR_NOME.get(base.toLowerCase()) ?? null;
  }

  // tests/.cache/shared/herdar-base-outland.mjs
  var CAMPOS_STAT = ["baseHp", "baseAtk", "baseDef", "baseSpAtk", "baseSpDef", "baseSpeed"];
  function herdarBaseOutland(lista) {
    const porId = new Map(lista.map((c) => [c.pokeId, c]));
    for (const esp of lista) {
      if (!isOutlandPokeId(esp.pokeId)) continue;
      const proprios = golpesDaPlanilhaDoOutland(esp.pokeId);
      if (proprios) esp.attacks = proprios;
      if (golpesDoOutlandFechados(esp.pokeId)) esp.golpesFechados = true;
      const baseDex = dexSpriteBaseOutland(esp.pokeId, esp.name);
      if (baseDex == null) continue;
      const base = porId.get(baseDex);
      if (!base) continue;
      for (const k of CAMPOS_STAT) {
        if (typeof base[k] === "number") esp[k] = base[k];
      }
      if (!proprios && Array.isArray(base.attacks)) esp.attacks = base.attacks.map((a) => ({ ...a }));
    }
  }

  // tests/.cache/shared/herdar-golpes-orre.mjs
  var herdar_golpes_orre_exports = {};
  __export(herdar_golpes_orre_exports, {
    herdarGolpesOrre: () => herdarGolpesOrre,
    nacionalDeOrre: () => nacionalDeOrre
  });
  var ORRE_MIN = 13e3;
  var ORRE_MAX = 14e3;
  var nacionalDeOrre = (pokeId) => pokeId >= ORRE_MIN && pokeId < ORRE_MAX ? pokeId - ORRE_MIN : null;
  function herdarGolpesOrre(lista) {
    const porId = new Map(lista.map((c) => [c.pokeId, c]));
    for (const esp of lista) {
      const baseId = nacionalDeOrre(esp?.pokeId ?? -1);
      if (baseId == null) continue;
      const base = porId.get(baseId);
      if (Array.isArray(base?.attacks)) esp.attacks = base.attacks.map((a) => ({ ...a }));
    }
  }

  // tests/.cache/shared/megas.mjs
  var megas_exports = {};
  __export(megas_exports, {
    GOLPE_MEGA_POR_TIPO: () => GOLPE_MEGA_POR_TIPO,
    MEGA2_LOOKTYPE_BASE: () => MEGA2_LOOKTYPE_BASE,
    MEGA2_POKE_BASE: () => MEGA2_POKE_BASE,
    MEGA2_SHINY_LOOKTYPE_BASE: () => MEGA2_SHINY_LOOKTYPE_BASE,
    MEGAS: () => MEGAS,
    MEGAS_SEGUNDAS: () => MEGAS_SEGUNDAS,
    MEGA_LOOKTYPE_BASE: () => MEGA_LOOKTYPE_BASE,
    MEGA_POKE_BASE: () => MEGA_POKE_BASE,
    MEGA_POKE_MAX: () => MEGA_POKE_MAX,
    MEGA_POR_DEX: () => MEGA_POR_DEX,
    MEGA_POR_POKEID: () => MEGA_POR_POKEID,
    MEGA_SEGUNDA_POR_DEX: () => MEGA_SEGUNDA_POR_DEX,
    MEGA_SHINY_LOOKTYPE_BASE: () => MEGA_SHINY_LOOKTYPE_BASE,
    TODAS_AS_MEGAS: () => TODAS_AS_MEGAS,
    criarEspeciesMega: () => criarEspeciesMega,
    dexDaMega: () => dexDaMega,
    elosMegaDaNota: () => elosMegaDaNota,
    especieBaseDaMega: () => especieBaseDaMega,
    golpeMegaDe: () => golpeMegaDe,
    isMegaPokeId: () => isMegaPokeId,
    looktypeDaMega: () => looktypeDaMega,
    looktypeMega: () => looktypeMega,
    looktypeMegaShiny: () => looktypeMegaShiny,
    looktypeShinyDaMega: () => looktypeShinyDaMega,
    megaDaEspecie: () => megaDaEspecie,
    megaPokeId: () => megaPokeId,
    megaPorPokeId: () => megaPorPokeId,
    megaTemShiny: () => megaTemShiny,
    megasDaEspecie: () => megasDaEspecie,
    pokeIdDaMega: () => pokeIdDaMega,
    totalBasesMega: () => totalBasesMega
  });

  // tests/.cache/shared/spawns-filtro.mjs
  var LENDARIO_DEX = /* @__PURE__ */ new Set([
    // 3ª Hoenn
    377,
    378,
    379,
    380,
    381,
    382,
    383,
    384,
    // 4ª Sinnoh
    480,
    481,
    482,
    483,
    484,
    485,
    486,
    487,
    488,
    // 5ª Unova
    638,
    639,
    640,
    641,
    642,
    643,
    644,
    645,
    646,
    // 6ª Kalos
    716,
    717,
    718,
    // 7ª Alola (Type: Null e Silvally, Tapu, nebulosa, UB, Necrozma). O Type: Null (#772) e o Silvally (#773) entraram
    // em 18/10/2026 (pedido do dono): são lendários, só pelo Mystic Ticket — tinham ficado de fora e viraram hunt de
    // Alola (Nv 75.000 e 120.550) quando a região abriu, em 17/10.
    772,
    773,
    785,
    786,
    787,
    788,
    789,
    790,
    791,
    792,
    793,
    794,
    795,
    796,
    797,
    798,
    799,
    800,
    803,
    804,
    805,
    806,
    // 8ª Galar (+ Crown Tundra, Legends Arceus)
    888,
    889,
    890,
    891,
    892,
    894,
    895,
    896,
    897,
    898,
    905,
    // 9ª Paldea (+ DLC / Teal Mask / Indigo Disk)
    1001,
    1002,
    1003,
    1004,
    1007,
    1008,
    1009,
    1010,
    1014,
    1015,
    1016,
    1017,
    1020,
    1021,
    1022,
    1023,
    1024
  ]);
  var MITICO_DEX = /* @__PURE__ */ new Set([
    385,
    386,
    489,
    490,
    491,
    492,
    493,
    647,
    648,
    649,
    719,
    720,
    721,
    801,
    802,
    807,
    808,
    809,
    893,
    1025
  ]);

  // tests/.cache/shared/escala-hunt-level.mjs
  function dexDe2(pokeId) {
    const id = Number(pokeId);
    if (!Number.isFinite(id) || id < 1) return id;
    if (id < 1e3) return id;
    if (id >= 13e3) return id % 1e3;
    if (isOutlandPokeId(id)) return id;
    return id;
  }

  // tests/.cache/shared/megas.mjs
  var MEGA_POKE_BASE = 3e3;
  var MEGA_POKE_MAX = 4e3;
  var MEGA_LOOKTYPE_BASE = 8e4;
  var MEGA_SHINY_LOOKTYPE_BASE = 85e3;
  var looktypeMega = (dex) => MEGA_LOOKTYPE_BASE + Number(dex);
  var looktypeMegaShiny = (dex) => MEGA_SHINY_LOOKTYPE_BASE + Number(dex);
  var megaPokeId = (dex) => MEGA_POKE_BASE + Number(dex);
  function isMegaPokeId(pokeId) {
    const id = Number(pokeId);
    return id > MEGA_POKE_BASE && id < MEGA_POKE_MAX || MEGA_SEGUNDA_POR_POKEID.has(id);
  }
  var MEGAS = [
    {
      dex: 3,
      slug: "venusaur",
      nome: "Mega Venusaur",
      pedra: "Venusaurite",
      icone: "venusaurite",
      tipos: ["GRASS", "POISON"],
      bases: { hp: 80, atk: 100, def: 123, spAtk: 122, spDef: 120, speed: 80 }
    },
    {
      dex: 6,
      slug: "charizard",
      nome: "Mega Charizard Y",
      pedra: "Charizardite Y",
      icone: "charizardite-y",
      tipos: ["FIRE", "FLYING"],
      bases: { hp: 78, atk: 104, def: 78, spAtk: 159, spDef: 115, speed: 100 }
    },
    {
      dex: 9,
      slug: "blastoise",
      nome: "Mega Blastoise",
      pedra: "Blastoisinite",
      icone: "blastoisinite",
      tipos: ["WATER"],
      bases: { hp: 79, atk: 103, def: 120, spAtk: 135, spDef: 115, speed: 78 }
    },
    {
      dex: 15,
      slug: "beedrill",
      nome: "Mega Beedrill",
      pedra: "Beedrillite",
      icone: "beedrillite",
      tipos: ["BUG", "POISON"],
      bases: { hp: 65, atk: 150, def: 40, spAtk: 15, spDef: 80, speed: 145 }
    },
    {
      dex: 18,
      slug: "pidgeot",
      nome: "Mega Pidgeot",
      pedra: "Pidgeotite",
      icone: "pidgeotite",
      tipos: ["NORMAL", "FLYING"],
      bases: { hp: 83, atk: 80, def: 80, spAtk: 135, spDef: 80, speed: 121 }
    },
    {
      // A forma Y, que chegou primeiro (25/09/2026). A X é outro sprite e outra pedra, e as duas
      // não cabem no mesmo `3000 + dex`: ela chegou em 18/10/2026 e mora em `MEGAS_SEGUNDAS`.
      dex: 26,
      slug: "raichu_y",
      nome: "Mega Raichu Y",
      pedra: "Raichunite Y",
      icone: "raichunite-y",
      tipos: ["ELECTRIC"],
      bases: { hp: 60, atk: 100, def: 55, spAtk: 160, spDef: 80, speed: 130 }
    },
    {
      dex: 36,
      slug: "clefable",
      nome: "Mega Clefable",
      pedra: "Clefablite",
      icone: "clefablite",
      tipos: ["FAIRY", "FLYING"],
      bases: { hp: 95, atk: 80, def: 93, spAtk: 135, spDef: 110, speed: 70 }
    },
    {
      dex: 65,
      slug: "alakazam",
      nome: "Mega Alakazam",
      pedra: "Alakazite",
      icone: "alakazite",
      tipos: ["PSYCHIC"],
      bases: { hp: 55, atk: 50, def: 65, spAtk: 175, spDef: 105, speed: 150 }
    },
    {
      dex: 71,
      slug: "victreebel",
      nome: "Mega Victreebel",
      pedra: "Victreebelite",
      icone: "victreebelite",
      tipos: ["GRASS", "POISON"],
      bases: { hp: 80, atk: 125, def: 85, spAtk: 135, spDef: 95, speed: 70 }
    },
    {
      dex: 80,
      slug: "slowbro",
      nome: "Mega Slowbro",
      pedra: "Slowbronite",
      icone: "slowbronite",
      tipos: ["WATER", "PSYCHIC"],
      bases: { hp: 95, atk: 75, def: 180, spAtk: 130, spDef: 80, speed: 30 }
    },
    {
      dex: 94,
      slug: "gengar",
      nome: "Mega Gengar",
      pedra: "Gengarite",
      icone: "gengarite",
      tipos: ["GHOST", "POISON"],
      bases: { hp: 60, atk: 65, def: 80, spAtk: 170, spDef: 95, speed: 130 }
    },
    {
      dex: 115,
      slug: "kangaskhan",
      nome: "Mega Kangaskhan",
      pedra: "Kangaskhanite",
      icone: "kangaskhanite",
      tipos: ["NORMAL"],
      bases: { hp: 105, atk: 125, def: 100, spAtk: 60, spDef: 100, speed: 100 }
    },
    {
      dex: 121,
      slug: "starmie",
      nome: "Mega Starmie",
      pedra: "Starminite",
      icone: "starminite",
      tipos: ["WATER", "PSYCHIC"],
      bases: { hp: 60, atk: 140, def: 105, spAtk: 130, spDef: 105, speed: 120 }
    },
    {
      dex: 127,
      slug: "pinsir",
      nome: "Mega Pinsir",
      pedra: "Pinsirite",
      icone: "pinsirite",
      tipos: ["BUG", "FLYING"],
      bases: { hp: 65, atk: 155, def: 120, spAtk: 65, spDef: 90, speed: 105 }
    },
    {
      dex: 130,
      slug: "gyarados",
      nome: "Mega Gyarados",
      pedra: "Gyaradosite",
      icone: "gyaradosite",
      tipos: ["WATER", "DARK"],
      bases: { hp: 95, atk: 155, def: 109, spAtk: 70, spDef: 130, speed: 81 }
    },
    {
      dex: 142,
      slug: "aerodactyl",
      nome: "Mega Aerodactyl",
      pedra: "Aerodactylite",
      icone: "aerodactylite",
      tipos: ["ROCK", "FLYING"],
      bases: { hp: 80, atk: 135, def: 85, spAtk: 70, spDef: 95, speed: 150 }
    },
    {
      dex: 149,
      slug: "dragonite",
      nome: "Mega Dragonite",
      pedra: "Dragonitite",
      icone: "dragonitite",
      tipos: ["DRAGON", "FLYING"],
      bases: { hp: 91, atk: 124, def: 115, spAtk: 145, spDef: 125, speed: 100 }
    },
    {
      // A Y fica com o `3000 + dex` (a primeira a ser desenhada); a X é a 2ª mega do Mewtwo —
      // ver `MEGAS_SEGUNDAS`.
      dex: 150,
      slug: "mewtwo_y",
      nome: "Mega Mewtwo Y",
      pedra: "Mewtwonite Y",
      icone: "mewtwonite-y",
      tipos: ["PSYCHIC"],
      bases: { hp: 106, atk: 150, def: 70, spAtk: 194, spDef: 120, speed: 140 }
    },
    {
      dex: 154,
      slug: "meganium",
      nome: "Mega Meganium",
      pedra: "Meganiumite",
      icone: "meganiumite",
      tipos: ["GRASS", "FAIRY"],
      bases: { hp: 80, atk: 92, def: 115, spAtk: 143, spDef: 115, speed: 80 }
    },
    {
      dex: 160,
      slug: "feraligatr",
      nome: "Mega Feraligatr",
      pedra: "Feraligite",
      icone: "feraligite",
      tipos: ["WATER", "DRAGON"],
      bases: { hp: 85, atk: 160, def: 125, spAtk: 89, spDef: 93, speed: 78 }
    },
    {
      dex: 181,
      slug: "ampharos",
      nome: "Mega Ampharos",
      pedra: "Ampharosite",
      icone: "ampharosite",
      tipos: ["ELECTRIC", "DRAGON"],
      bases: { hp: 90, atk: 95, def: 105, spAtk: 165, spDef: 110, speed: 45 }
    },
    {
      dex: 208,
      slug: "steelix",
      nome: "Mega Steelix",
      pedra: "Steelixite",
      icone: "steelixite",
      tipos: ["STEEL", "GROUND"],
      bases: { hp: 75, atk: 125, def: 230, spAtk: 55, spDef: 95, speed: 30 }
    },
    {
      dex: 212,
      slug: "scizor",
      nome: "Mega Scizor",
      pedra: "Scizorite",
      icone: "scizorite",
      tipos: ["BUG", "STEEL"],
      bases: { hp: 70, atk: 150, def: 140, spAtk: 65, spDef: 100, speed: 75 }
    },
    {
      dex: 214,
      slug: "heracross",
      nome: "Mega Heracross",
      pedra: "Heracronite",
      icone: "heracronite",
      tipos: ["BUG", "FIGHTING"],
      bases: { hp: 80, atk: 185, def: 115, spAtk: 40, spDef: 105, speed: 75 }
    },
    {
      dex: 227,
      slug: "skarmory",
      nome: "Mega Skarmory",
      pedra: "Skarmorite",
      icone: "skarmorite",
      tipos: ["STEEL", "FLYING"],
      bases: { hp: 65, atk: 140, def: 110, spAtk: 40, spDef: 100, speed: 110 }
    },
    {
      dex: 229,
      slug: "houndoom",
      nome: "Mega Houndoom",
      pedra: "Houndoominite",
      icone: "houndoominite",
      tipos: ["DARK", "FIRE"],
      bases: { hp: 75, atk: 90, def: 90, spAtk: 140, spDef: 90, speed: 115 }
    },
    {
      dex: 248,
      slug: "tyranitar",
      nome: "Mega Tyranitar",
      pedra: "Tyranitarite",
      icone: "tyranitarite",
      tipos: ["ROCK", "DARK"],
      bases: { hp: 100, atk: 164, def: 150, spAtk: 95, spDef: 120, speed: 71 }
    },
    {
      dex: 254,
      slug: "sceptile",
      nome: "Mega Sceptile",
      pedra: "Sceptilite",
      icone: "sceptilite",
      tipos: ["GRASS", "DRAGON"],
      bases: { hp: 70, atk: 110, def: 75, spAtk: 145, spDef: 85, speed: 145 }
    },
    {
      dex: 257,
      slug: "blaziken",
      nome: "Mega Blaziken",
      pedra: "Blazikenite",
      icone: "blazikenite",
      tipos: ["FIRE", "FIGHTING"],
      bases: { hp: 80, atk: 160, def: 80, spAtk: 130, spDef: 80, speed: 100 }
    },
    {
      dex: 260,
      slug: "swampert",
      nome: "Mega Swampert",
      pedra: "Swampertite",
      icone: "swampertite",
      tipos: ["WATER", "GROUND"],
      bases: { hp: 100, atk: 150, def: 110, spAtk: 95, spDef: 110, speed: 70 }
    },
    {
      dex: 282,
      slug: "gardevoir",
      nome: "Mega Gardevoir",
      pedra: "Gardevoirite",
      icone: "gardevoirite",
      tipos: ["PSYCHIC", "FAIRY"],
      bases: { hp: 68, atk: 85, def: 65, spAtk: 165, spDef: 135, speed: 100 }
    },
    {
      dex: 302,
      slug: "sableye",
      nome: "Mega Sableye",
      pedra: "Sablenite",
      icone: "sablenite",
      tipos: ["DARK", "GHOST"],
      bases: { hp: 50, atk: 85, def: 125, spAtk: 85, spDef: 115, speed: 20 }
    },
    {
      dex: 303,
      slug: "mawile",
      nome: "Mega Mawile",
      pedra: "Mawilite",
      icone: "mawilite",
      tipos: ["STEEL", "FAIRY"],
      bases: { hp: 50, atk: 105, def: 125, spAtk: 55, spDef: 95, speed: 50 }
    },
    {
      dex: 306,
      slug: "aggron",
      nome: "Mega Aggron",
      pedra: "Aggronite",
      icone: "aggronite",
      // STEEL puro, a oficial. De 03/10 a 21/10/2026 guardou o ROCK do Aggron (planilha "Megas
      // 03-10"), para seguir usando os TMs de ROCK da base; a "Novas Megas Arrumadas" (21/10) a
      // escreve só STEEL e tira todos os golpes de ROCK. O TM de ROCK já aplicado fica (o Z-Move
      // mora no pokemon, como o FLYING do Gyarados que vira Mega Gyarados), só sem STAB.
      tipos: ["STEEL"],
      bases: { hp: 70, atk: 140, def: 230, spAtk: 60, spDef: 80, speed: 50 }
    },
    {
      dex: 308,
      slug: "medicham",
      nome: "Mega Medicham",
      pedra: "Medichamite",
      icone: "medichamite",
      tipos: ["FIGHTING", "PSYCHIC"],
      bases: { hp: 60, atk: 100, def: 85, spAtk: 80, spDef: 85, speed: 100 }
    },
    {
      dex: 310,
      slug: "manectric",
      nome: "Mega Manectric",
      pedra: "Manectite",
      icone: "manectite",
      tipos: ["ELECTRIC"],
      bases: { hp: 70, atk: 75, def: 80, spAtk: 135, spDef: 80, speed: 135 }
    },
    {
      dex: 319,
      slug: "sharpedo",
      nome: "Mega Sharpedo",
      pedra: "Sharpedonite",
      icone: "sharpedonite",
      tipos: ["WATER", "DARK"],
      bases: { hp: 70, atk: 140, def: 70, spAtk: 110, spDef: 65, speed: 105 }
    },
    {
      dex: 323,
      slug: "camerupt",
      nome: "Mega Camerupt",
      pedra: "Cameruptite",
      icone: "cameruptite",
      tipos: ["FIRE", "GROUND"],
      bases: { hp: 70, atk: 120, def: 100, spAtk: 145, spDef: 105, speed: 20 }
    },
    {
      dex: 334,
      slug: "altaria",
      nome: "Mega Altaria",
      pedra: "Altarianite",
      icone: "altarianite",
      tipos: ["DRAGON", "FAIRY"],
      bases: { hp: 75, atk: 110, def: 110, spAtk: 110, spDef: 105, speed: 80 }
    },
    {
      dex: 354,
      slug: "banette",
      nome: "Mega Banette",
      pedra: "Banettite",
      icone: "banettite",
      tipos: ["GHOST"],
      bases: { hp: 64, atk: 165, def: 75, spAtk: 93, spDef: 83, speed: 75 }
    },
    {
      dex: 358,
      slug: "chimecho",
      nome: "Mega Chimecho",
      pedra: "Chimechite",
      icone: "chimechite",
      tipos: ["PSYCHIC", "STEEL"],
      bases: { hp: 75, atk: 50, def: 110, spAtk: 135, spDef: 120, speed: 65 }
    },
    {
      dex: 359,
      slug: "absol",
      nome: "Mega Absol",
      pedra: "Absolite",
      icone: "absolite",
      tipos: ["DARK"],
      bases: { hp: 65, atk: 150, def: 60, spAtk: 115, spDef: 60, speed: 115 }
    },
    {
      dex: 362,
      slug: "glalie",
      nome: "Mega Glalie",
      pedra: "Glalitite",
      icone: "glalitite",
      tipos: ["ICE"],
      bases: { hp: 80, atk: 120, def: 80, spAtk: 120, spDef: 80, speed: 100 }
    },
    {
      dex: 373,
      slug: "salamence",
      nome: "Mega Salamence",
      pedra: "Salamencite",
      icone: "salamencite",
      tipos: ["DRAGON", "FLYING"],
      bases: { hp: 95, atk: 145, def: 130, spAtk: 120, spDef: 90, speed: 120 }
    },
    {
      dex: 376,
      slug: "metagross",
      nome: "Mega Metagross",
      pedra: "Metagrossite",
      icone: "metagrossite",
      tipos: ["STEEL", "PSYCHIC"],
      bases: { hp: 80, atk: 145, def: 150, spAtk: 105, spDef: 110, speed: 110 }
    },
    {
      dex: 380,
      slug: "latias",
      nome: "Mega Latias",
      pedra: "Latiasite",
      icone: "latiasite",
      tipos: ["DRAGON", "PSYCHIC"],
      bases: { hp: 80, atk: 100, def: 120, spAtk: 140, spDef: 150, speed: 110 }
    },
    {
      dex: 381,
      slug: "latios",
      nome: "Mega Latios",
      pedra: "Latiosite",
      icone: "latiosite",
      tipos: ["DRAGON", "PSYCHIC"],
      bases: { hp: 80, atk: 130, def: 100, spAtk: 160, spDef: 120, speed: 110 }
    },
    {
      // Reversão Primal, e não mega: no jogo de origem a Blue Orb não é Mega Stone. Aqui ela segue
      // o mesmo caminho (pedra, Professor, fragmentos) porque o efeito na ficha é o mesmo.
      dex: 382,
      slug: "kyogre_primal",
      nome: "Primal Kyogre",
      pedra: "Blue Orb",
      icone: "blue-orb",
      tipos: ["WATER"],
      bases: { hp: 100, atk: 150, def: 90, spAtk: 180, spDef: 160, speed: 90 }
    },
    {
      // Reversão Primal, como a do Kyogre, com a Red Orb.
      dex: 383,
      slug: "groudon_primal",
      nome: "Primal Groudon",
      pedra: "Red Orb",
      icone: "red-orb",
      tipos: ["GROUND", "FIRE"],
      bases: { hp: 100, atk: 180, def: 160, spAtk: 150, spDef: 90, speed: 90 }
    },
    {
      // A Jade Orb no lugar de pedra: o Rayquaza não tem Mega Stone no jogo de origem (megaevolui
      // pelo Dragon Ascent). Aqui ele precisa de um item, como todo mundo.
      dex: 384,
      slug: "rayquaza",
      nome: "Mega Rayquaza",
      pedra: "Jade Orb",
      icone: "jade-orb",
      tipos: ["DRAGON", "FLYING"],
      bases: { hp: 105, atk: 180, def: 100, spAtk: 180, spDef: 100, speed: 115 }
    },
    {
      dex: 398,
      slug: "staraptor",
      nome: "Mega Staraptor",
      pedra: "Staraptite",
      icone: "staraptite",
      tipos: ["FIGHTING", "FLYING"],
      bases: { hp: 85, atk: 140, def: 100, spAtk: 60, spDef: 90, speed: 110 }
    },
    {
      dex: 428,
      slug: "lopunny",
      nome: "Mega Lopunny",
      pedra: "Lopunnite",
      icone: "lopunnite",
      tipos: ["NORMAL", "FIGHTING"],
      bases: { hp: 65, atk: 136, def: 94, spAtk: 54, spDef: 96, speed: 135 }
    },
    {
      dex: 445,
      slug: "garchomp",
      nome: "Mega Garchomp",
      pedra: "Garchompite",
      icone: "garchompite",
      tipos: ["DRAGON", "GROUND"],
      bases: { hp: 108, atk: 170, def: 115, spAtk: 120, spDef: 95, speed: 92 }
    },
    {
      dex: 448,
      slug: "lucario",
      nome: "Mega Lucario",
      pedra: "Lucarionite",
      icone: "lucarionite",
      tipos: ["FIGHTING", "STEEL"],
      bases: { hp: 70, atk: 145, def: 88, spAtk: 140, spDef: 70, speed: 112 }
    },
    {
      dex: 460,
      slug: "abomasnow",
      nome: "Mega Abomasnow",
      pedra: "Abomasite",
      icone: "abomasite",
      tipos: ["GRASS", "ICE"],
      bases: { hp: 90, atk: 132, def: 105, spAtk: 132, spDef: 105, speed: 30 }
    },
    {
      dex: 475,
      slug: "gallade",
      nome: "Mega Gallade",
      pedra: "Galladite",
      icone: "galladite",
      tipos: ["PSYCHIC", "FIGHTING"],
      bases: { hp: 68, atk: 165, def: 95, spAtk: 65, spDef: 115, speed: 110 }
    },
    {
      dex: 478,
      slug: "froslass",
      nome: "Mega Froslass",
      pedra: "Froslassite",
      icone: "froslassite",
      tipos: ["ICE", "GHOST"],
      bases: { hp: 70, atk: 80, def: 70, spAtk: 140, spDef: 100, speed: 120 }
    },
    {
      dex: 485,
      slug: "heatran",
      nome: "Mega Heatran",
      pedra: "Heatranite",
      icone: "heatranite",
      tipos: ["FIRE", "STEEL"],
      bases: { hp: 91, atk: 120, def: 106, spAtk: 175, spDef: 141, speed: 67 }
    },
    {
      dex: 491,
      slug: "darkrai",
      nome: "Mega Darkrai",
      pedra: "Darkranite",
      icone: "darkranite",
      tipos: ["DARK"],
      bases: { hp: 70, atk: 120, def: 130, spAtk: 165, spDef: 130, speed: 85 }
    },
    {
      dex: 500,
      slug: "emboar",
      nome: "Mega Emboar",
      pedra: "Emboarite",
      icone: "emboarite",
      tipos: ["FIRE", "FIGHTING"],
      bases: { hp: 110, atk: 148, def: 75, spAtk: 110, spDef: 110, speed: 75 }
    },
    {
      dex: 530,
      slug: "excadrill",
      nome: "Mega Excadrill",
      pedra: "Excadrite",
      icone: "excadrite",
      tipos: ["GROUND", "STEEL"],
      bases: { hp: 110, atk: 165, def: 100, spAtk: 65, spDef: 65, speed: 103 }
    },
    {
      dex: 531,
      slug: "audino",
      nome: "Mega Audino",
      pedra: "Audinite",
      icone: "audinite",
      tipos: ["NORMAL", "FAIRY"],
      bases: { hp: 103, atk: 60, def: 126, spAtk: 80, spDef: 126, speed: 50 }
    },
    {
      dex: 545,
      slug: "scolipede",
      nome: "Mega Scolipede",
      pedra: "Scolipite",
      icone: "scolipite",
      tipos: ["BUG", "POISON"],
      bases: { hp: 60, atk: 140, def: 149, spAtk: 75, spDef: 99, speed: 62 }
    },
    {
      dex: 560,
      slug: "scrafty",
      nome: "Mega Scrafty",
      pedra: "Scraftinite",
      icone: "scraftinite",
      tipos: ["DARK", "FIGHTING"],
      bases: { hp: 65, atk: 130, def: 135, spAtk: 55, spDef: 135, speed: 68 }
    },
    {
      dex: 604,
      slug: "eelektross",
      nome: "Mega Eelektross",
      pedra: "Eelektrossite",
      icone: "eelektrossite",
      tipos: ["ELECTRIC"],
      bases: { hp: 85, atk: 145, def: 80, spAtk: 135, spDef: 90, speed: 80 }
    },
    {
      dex: 609,
      slug: "chandelure",
      nome: "Mega Chandelure",
      pedra: "Chandelurite",
      icone: "chandelurite",
      tipos: ["GHOST", "FIRE"],
      bases: { hp: 60, atk: 75, def: 110, spAtk: 175, spDef: 110, speed: 90 }
    },
    {
      dex: 623,
      slug: "golurk",
      nome: "Mega Golurk",
      pedra: "Golurkite",
      icone: "golurkite",
      tipos: ["GROUND", "GHOST"],
      bases: { hp: 89, atk: 159, def: 105, spAtk: 70, spDef: 105, speed: 55 }
    },
    {
      dex: 652,
      slug: "chesnaught",
      nome: "Mega Chesnaught",
      pedra: "Chesnaughtite",
      icone: "chesnaughtite",
      tipos: ["GRASS", "FIGHTING"],
      bases: { hp: 88, atk: 137, def: 172, spAtk: 74, spDef: 115, speed: 44 }
    },
    {
      dex: 655,
      slug: "delphox",
      nome: "Mega Delphox",
      pedra: "Delphoxite",
      icone: "delphoxite",
      tipos: ["FIRE", "PSYCHIC"],
      bases: { hp: 75, atk: 69, def: 72, spAtk: 159, spDef: 125, speed: 134 }
    },
    {
      dex: 658,
      slug: "greninja",
      nome: "Mega Greninja",
      pedra: "Greninjite",
      icone: "greninjite",
      tipos: ["WATER", "DARK"],
      bases: { hp: 72, atk: 125, def: 77, spAtk: 133, spDef: 81, speed: 142 }
    },
    {
      dex: 668,
      slug: "pyroar",
      nome: "Mega Pyroar",
      pedra: "Pyroarite",
      icone: "pyroarite",
      tipos: ["FIRE", "NORMAL"],
      bases: { hp: 86, atk: 88, def: 92, spAtk: 129, spDef: 86, speed: 126 }
    },
    {
      // A mega da Floette de flor eterna no jogo de origem; aqui, da Floette do catálogo.
      dex: 670,
      slug: "floette",
      nome: "Mega Floette",
      pedra: "Floettite",
      icone: "floettite",
      tipos: ["FAIRY"],
      bases: { hp: 74, atk: 85, def: 87, spAtk: 155, spDef: 148, speed: 102 }
    },
    {
      dex: 678,
      slug: "meowstic",
      nome: "Mega Meowstic",
      pedra: "Meowsticite",
      icone: "meowsticite",
      tipos: ["PSYCHIC"],
      bases: { hp: 74, atk: 48, def: 76, spAtk: 143, spDef: 101, speed: 124 }
    },
    {
      dex: 687,
      slug: "malamar",
      nome: "Mega Malamar",
      pedra: "Malamarite",
      icone: "malamarite",
      tipos: ["DARK", "PSYCHIC"],
      bases: { hp: 86, atk: 102, def: 88, spAtk: 98, spDef: 120, speed: 88 }
    },
    {
      dex: 689,
      slug: "barbaracle",
      nome: "Mega Barbaracle",
      pedra: "Barbaracite",
      icone: "barbaracite",
      tipos: ["ROCK", "FIGHTING"],
      bases: { hp: 72, atk: 140, def: 130, spAtk: 64, spDef: 106, speed: 88 }
    },
    {
      dex: 691,
      slug: "dragalge",
      nome: "Mega Dragalge",
      pedra: "Dragalgite",
      icone: "dragalgite",
      tipos: ["POISON", "DRAGON"],
      bases: { hp: 65, atk: 85, def: 105, spAtk: 132, spDef: 163, speed: 44 }
    },
    {
      dex: 701,
      slug: "hawlucha",
      nome: "Mega Hawlucha",
      pedra: "Hawluchanite",
      icone: "hawluchanite",
      tipos: ["FIGHTING", "FLYING"],
      bases: { hp: 78, atk: 137, def: 100, spAtk: 74, spDef: 93, speed: 118 }
    },
    {
      // A partir do Zygarde COMPLETO (#9718), a última forma da cadeia 10% → 50% → Completo
      // (`evolucoes-cruzadas.mjs`) — pedido do dono em 30/10/2026: até ali a mega saía do 50% (#718),
      // a segunda forma. Nem o 10% nem o 50% megaevoluem.
      dex: 718,
      base: 9718,
      slug: "zygarde",
      nome: "Mega Zygarde",
      pedra: "Zygardite",
      icone: "zygardite",
      tipos: ["DRAGON", "GROUND"],
      bases: { hp: 216, atk: 70, def: 91, spAtk: 216, spDef: 85, speed: 100 }
    },
    {
      dex: 719,
      slug: "diancie",
      nome: "Mega Diancie",
      pedra: "Diancite",
      icone: "diancite",
      tipos: ["ROCK", "FAIRY"],
      bases: { hp: 50, atk: 160, def: 110, spAtk: 160, spDef: 110, speed: 110 }
    },
    {
      dex: 740,
      slug: "crabominable",
      nome: "Mega Crabominable",
      pedra: "Crabominite",
      icone: "crabominite",
      tipos: ["FIGHTING", "ICE"],
      bases: { hp: 97, atk: 157, def: 122, spAtk: 62, spDef: 107, speed: 33 }
    },
    {
      dex: 768,
      slug: "golisopod",
      nome: "Mega Golisopod",
      pedra: "Golisopite",
      icone: "golisopite",
      tipos: ["BUG", "STEEL"],
      bases: { hp: 75, atk: 150, def: 175, spAtk: 70, spDef: 120, speed: 40 }
    },
    {
      dex: 780,
      slug: "drampa",
      nome: "Mega Drampa",
      pedra: "Drampanite",
      icone: "drampanite",
      tipos: ["NORMAL", "DRAGON"],
      bases: { hp: 78, atk: 85, def: 110, spAtk: 160, spDef: 116, speed: 36 }
    },
    {
      dex: 801,
      slug: "magearna",
      nome: "Mega Magearna",
      pedra: "Magearnite",
      icone: "magearnite",
      tipos: ["STEEL", "FAIRY"],
      bases: { hp: 80, atk: 125, def: 115, spAtk: 170, spDef: 115, speed: 95 }
    },
    {
      dex: 807,
      slug: "zeraora",
      nome: "Mega Zeraora",
      pedra: "Zeraorite",
      icone: "zeraorite",
      tipos: ["ELECTRIC"],
      bases: { hp: 88, atk: 157, def: 75, spAtk: 147, spDef: 80, speed: 153 }
    },
    {
      dex: 870,
      slug: "falinks",
      nome: "Mega Falinks",
      pedra: "Falinksite",
      icone: "falinksite",
      tipos: ["FIGHTING"],
      bases: { hp: 65, atk: 135, def: 135, spAtk: 70, spDef: 65, speed: 100 }
    },
    {
      dex: 952,
      slug: "scovillain",
      nome: "Mega Scovillain",
      pedra: "Scovillainite",
      icone: "scovillainite",
      tipos: ["GRASS", "FIRE"],
      bases: { hp: 65, atk: 138, def: 85, spAtk: 138, spDef: 85, speed: 75 }
    },
    {
      dex: 970,
      slug: "glimmora",
      nome: "Mega Glimmora",
      pedra: "Glimmoranite",
      icone: "glimmoranite",
      tipos: ["ROCK", "POISON"],
      bases: { hp: 83, atk: 90, def: 105, spAtk: 150, spDef: 96, speed: 101 }
    },
    {
      dex: 978,
      slug: "tatsugiri",
      nome: "Mega Tatsugiri",
      pedra: "Tatsugirinite",
      icone: "tatsugirinite",
      tipos: ["DRAGON", "WATER"],
      bases: { hp: 68, atk: 65, def: 90, spAtk: 135, spDef: 125, speed: 92 }
    },
    {
      dex: 998,
      slug: "baxcalibur",
      nome: "Mega Baxcalibur",
      pedra: "Baxcalibrite",
      icone: "baxcalibrite",
      tipos: ["DRAGON", "ICE"],
      bases: { hp: 115, atk: 175, def: 117, spAtk: 105, spDef: 101, speed: 87 }
    }
  ];
  var MEGAS_SEGUNDAS = [
    {
      dex: 6,
      segunda: true,
      slug: "charizard_x",
      nome: "Mega Charizard X",
      pedra: "Charizardite X",
      icone: "charizardite-x",
      tipos: ["FIRE", "DRAGON"],
      bases: { hp: 78, atk: 130, def: 111, spAtk: 130, spDef: 85, speed: 100 }
    },
    {
      dex: 26,
      segunda: true,
      slug: "raichu_x",
      nome: "Mega Raichu X",
      pedra: "Raichunite X",
      icone: "raichunite-x",
      tipos: ["ELECTRIC"],
      bases: { hp: 60, atk: 135, def: 95, spAtk: 90, spDef: 95, speed: 110 }
    },
    {
      dex: 150,
      segunda: true,
      slug: "mewtwo_x",
      nome: "Mega Mewtwo X",
      pedra: "Mewtwonite X",
      icone: "mewtwonite-x",
      tipos: ["PSYCHIC", "FIGHTING"],
      bases: { hp: 106, atk: 190, def: 100, spAtk: 154, spDef: 100, speed: 130 }
    },
    {
      dex: 445,
      segunda: true,
      slug: "garchomp_z",
      nome: "Mega Garchomp Z",
      pedra: "Garchompite Z",
      icone: "garchompite-z",
      tipos: ["DRAGON"],
      bases: { hp: 108, atk: 130, def: 85, spAtk: 141, spDef: 85, speed: 151 }
    },
    {
      dex: 448,
      segunda: true,
      slug: "lucario_z",
      nome: "Mega Lucario Z",
      pedra: "Lucarionite Z",
      icone: "lucarionite-z",
      tipos: ["FIGHTING", "STEEL"],
      bases: { hp: 70, atk: 100, def: 70, spAtk: 164, spDef: 70, speed: 151 }
    }
  ];
  var MEGA2_POKE_BASE = 9e3;
  var MEGA2_LOOKTYPE_BASE = 83e3;
  var MEGA2_SHINY_LOOKTYPE_BASE = 88e3;
  var TODAS_AS_MEGAS = [...MEGAS, ...MEGAS_SEGUNDAS];
  var pokeIdDaMega = (meta) => (meta.segunda ? MEGA2_POKE_BASE : MEGA_POKE_BASE) + Number(meta.dex);
  var looktypeDaMega = (meta) => (meta.segunda ? MEGA2_LOOKTYPE_BASE : MEGA_LOOKTYPE_BASE) + Number(meta.dex);
  var looktypeShinyDaMega = (meta) => (meta.segunda ? MEGA2_SHINY_LOOKTYPE_BASE : MEGA_SHINY_LOOKTYPE_BASE) + Number(meta.dex);
  var MEGA_POR_DEX = new Map(MEGAS.map((m) => [m.dex, m]));
  var MEGA_SEGUNDA_POR_DEX = new Map(MEGAS_SEGUNDAS.map((m) => [m.dex, m]));
  var MEGA_POR_POKEID = new Map(TODAS_AS_MEGAS.map((m) => [pokeIdDaMega(m), m]));
  var MEGA_SEGUNDA_POR_POKEID = new Map(MEGAS_SEGUNDAS.map((m) => [pokeIdDaMega(m), m]));
  var MEGA_POR_BASE = new Map(TODAS_AS_MEGAS.filter((m) => m.base != null).map((m) => [m.base, m]));
  var especieBaseDaMega = (meta) => meta ? meta.base ?? meta.dex : null;
  function dexDaMega(pokeId) {
    const id = Number(pokeId);
    if (!Number.isFinite(id) || isOutlandPokeId(id) || isMegaPokeId(id)) return null;
    const daBase = MEGA_POR_BASE.get(id);
    if (daBase) return daBase.dex;
    const dex = dexDe2(id);
    const meta = MEGA_POR_DEX.get(dex);
    return meta && meta.base == null ? dex : null;
  }
  function megaDaEspecie(pokeId, segunda = false) {
    const dex = dexDaMega(pokeId);
    if (dex == null) return null;
    return (segunda ? MEGA_SEGUNDA_POR_DEX.get(dex) : MEGA_POR_DEX.get(dex)) ?? null;
  }
  function megasDaEspecie(pokeId) {
    const dex = dexDaMega(pokeId);
    if (dex == null) return [];
    return [MEGA_POR_DEX.get(dex), MEGA_SEGUNDA_POR_DEX.get(dex)].filter(Boolean);
  }
  var megaPorPokeId = (pokeId) => MEGA_POR_POKEID.get(Number(pokeId)) ?? null;
  var megaTemShiny = (meta) => !!meta && !meta.semShiny;
  var totalBasesMega = (m) => m.bases.hp + m.bases.atk + m.bases.def + m.bases.spAtk + m.bases.spDef + m.bases.speed;
  var GOLPE_MEGA_POR_TIPO = {
    NORMAL: "Mega Onslaught",
    FIRE: "Mega Inferno",
    WATER: "Mega Maelstrom",
    GRASS: "Mega Bloom",
    ELECTRIC: "Mega Thunderstorm",
    ICE: "Mega Permafrost",
    FIGHTING: "Mega Rampage",
    POISON: "Mega Venom",
    GROUND: "Mega Earthshock",
    FLYING: "Mega Tempest",
    PSYCHIC: "Mega Mindbreak",
    BUG: "Mega Swarm",
    ROCK: "Mega Landslide",
    GHOST: "Mega Eclipse",
    DRAGON: "Mega Cataclysm",
    DARK: "Mega Abyss",
    STEEL: "Mega Forge",
    FAIRY: "Mega Radiance"
  };
  function golpeMegaDe(meta) {
    const tipo = meta?.tipos?.[0];
    const nome = GOLPE_MEGA_POR_TIPO[tipo];
    if (!nome) return null;
    return {
      name: nome,
      power: 600,
      type: tipo,
      category: (meta.bases.atk ?? 0) >= (meta.bases.spAtk ?? 0) ? "PHYSICAL" : "SPECIAL",
      cooldownMs: COOLDOWN_MEGA_MS,
      learnLevel: 1,
      // Ver o cabeçalho: sem esta marca o golpe volta para os 60 s de todo golpe de 600.
      cdFixo: true,
      // O que a tela usa para pintar o selo de "golpe da mega" na ficha.
      golpeMega: true
    };
  }
  function criarEspeciesMega(lista) {
    const porId = new Map(lista.map((c) => [c.pokeId, c]));
    const saida = [];
    for (const m of TODAS_AS_MEGAS) {
      const base = porId.get(especieBaseDaMega(m));
      if (!base) continue;
      const pokeId = pokeIdDaMega(m);
      saida.push({
        ...base,
        pokeId,
        name: m.nome,
        looktype: looktypeDaMega(m),
        description: `a ${m.nome.toLowerCase()}`,
        type1: m.tipos[0],
        type2: m.tipos[1] ?? null,
        baseHp: m.bases.hp,
        baseAtk: m.bases.atk,
        baseDef: m.bases.def,
        baseSpAtk: m.bases.spAtk,
        baseSpDef: m.bases.spDef,
        baseSpeed: m.bases.speed,
        evolvesToId: 0,
        evolveLevel: 0,
        // O teto de quem NÃO evolui (`TETO_CAPTURA_MAX` em `teto-captura.mjs`, aqui sem o import para
        // não fechar um ciclo de módulos). Até 16/10 toda mega era de última evolução e herdava o 100 da
        // base; a Mega Floette é a primeira de uma forma do meio (Flabébé → Floette → Florges), e
        // herdaria o 40 da Floette — o que uma segunda passagem do teto desfaria.
        tetoCaptura: 100,
        // `loot` e `attacks` viriam por REFERÊNCIA no espalhamento acima, e passagens do boot
        // ainda dão `push` neles (ver `aplicarDropsNossos`). Copiar aqui é o que impede a mega
        // de escrever na ficha do Gengar de verdade.
        loot: (base.loot ?? []).map((l) => ({ ...l })),
        // O moveset da planilha, quando a mega está nela; senão o da base, CÓPIA EXATA, mais o
        // golpe de mega — e nada mais. Ver o cabeçalho de `GOLPE_MEGA_POR_TIPO` para por que a
        // cópia não é rederivada dos tipos novos.
        attacks: golpesDaPlanilhaDaMega(pokeId) ?? [
          ...(base.attacks ?? []).map((a) => ({ ...a })),
          ...golpeMegaDe(m) ? [golpeMegaDe(m)] : []
        ],
        megaDeDex: m.dex,
        ...m.segunda ? { megaSegunda: true } : {}
      });
    }
    return saida;
  }
  function elosMegaDaNota(lista) {
    const existe = new Set(lista.map((c) => c.pokeId));
    const elos = [];
    for (const m of TODAS_AS_MEGAS) {
      const mega = pokeIdDaMega(m);
      const base = especieBaseDaMega(m);
      if (!existe.has(base) || !existe.has(mega)) continue;
      elos.push([base, mega]);
    }
    return elos;
  }

  // tests/.cache/shared/evolucoes-cruzadas.mjs
  var evolucoes_cruzadas_exports = {};
  __export(evolucoes_cruzadas_exports, {
    CADEIAS_TRUNCADAS: () => CADEIAS_TRUNCADAS,
    EVOLUCOES_ENTRE_GERACOES: () => EVOLUCOES_ENTRE_GERACOES,
    NIVEIS_POR_ONDE_O_ALVO_APARECE: () => NIVEIS_POR_ONDE_O_ALVO_APARECE,
    aplicarCadeiasTruncadas: () => aplicarCadeiasTruncadas,
    aplicarElosEvolutivos: () => aplicarElosEvolutivos,
    aplicarEvolucoesEntreGeracoes: () => aplicarEvolucoesEntreGeracoes,
    corrigirNiveisDeEvolucao: () => corrigirNiveisDeEvolucao,
    normalizarEvolveLevel600: () => normalizarEvolveLevel600
  });
  var CADEIAS_TRUNCADAS = {
    288: { para: 289, nivel: 1250 },
    // Vigoroth → Slaking
    13288: { para: 289, nivel: 1250 },
    372: { para: 373, nivel: 1250 },
    // Shelgon  → Salamence
    13372: { para: 373, nivel: 1250 },
    375: { para: 376, nivel: 1250 },
    // Metang   → Metagross (espelho truncava no 600→100)
    13375: { para: 376, nivel: 1250 }
    // Metang Hoenn → Metagross
  };
  var EVOLUCOES_ENTRE_GERACOES = {
    57: { para: 979, nivel: 114800 },
    // Primeape   Kanto → Annihilape Paldea
    82: { para: 462, nivel: 1500 },
    // Magneton   Kanto → Magnezone  Sinnoh
    108: { para: 463, nivel: 1500 },
    // Lickitung  Kanto → Lickilicky Sinnoh
    176: { para: 468, nivel: 1500 },
    // Togetic    Johto → Togekiss   Sinnoh
    190: { para: 424, nivel: 1250 },
    // Aipom      Johto → Ambipom    Sinnoh
    193: { para: 469, nivel: 1500 },
    // Yanma      Johto → Yanmega    Sinnoh
    198: { para: 430, nivel: 1500 },
    // Murkrow    Johto → Honchkrow  Sinnoh
    200: { para: 429, nivel: 1250 },
    // Misdreavus Johto → Mismagius  Sinnoh
    207: { para: 472, nivel: 1500 },
    // Gligar     Johto → Gliscor    Sinnoh
    215: { para: 461, nivel: 1500 },
    // Sneasel    Johto → Weavile    Sinnoh
    217: { para: 901, nivel: 58900 },
    // Ursaring   Johto → Ursaluna   Galar
    221: { para: 473, nivel: 1500 },
    // Piloswine  Johto → Mamoswine  Sinnoh
    233: { para: 474, nivel: 1500 },
    // Porygon2   Johto → Porygon-Z  Sinnoh
    // Makuhita → Hariyama não atravessa geração: é a mesma cadeia truncada do Vigoroth
    // (`CADEIAS_TRUNCADAS` acima), e o espelho de Hoenn tem as duas metades. Vale para o id
    // do espelho e para o clone 13xxx, que é o que a hunt de Hoenn usa.
    296: { para: 297, nivel: 550 },
    13296: { para: 297, nivel: 550 },
    // --- 18/10/2026 (ver o cabeçalho). O nível é o do alvo — o marcador da hunt dele, ou o
    // `huntLevel` de quem não tem hunt —, e o teto de captura o reescreve no boot (`teto-captura.mjs`).
    203: { para: 981, nivel: 46150 },
    // Girafarig   Johto → Farigiraf        Paldea
    206: { para: 982, nivel: 46150 },
    // Dunsparce   Johto → Dudunsparce      Paldea
    234: { para: 899, nivel: 27500 },
    // Stantler    Johto → Wyrdeer          Hisui
    808: { para: 809, nivel: 108e3 },
    // Meltan            → Melmetal
    647: { para: 8647, nivel: 8600 },
    // Keldeo            → Keldeo Resolute
    8718: { para: 718, nivel: 1e4 },
    // Zygarde 10%       → Zygarde
    718: { para: 9718, nivel: 21200 },
    // Zygarde           → Zygarde Complete
    746: { para: 8746, nivel: 84650 },
    // Wishiwashi        → Wishiwashi School
    // --- As outras FORMAS como evolução (pedido do dono, 18/10/2026): as de lendário, que não têm hunt
    // nem saem do Mystic Ticket e por isso ninguém conseguia ter, e as três de batalha que só vinham da
    // hunt. A pedra é a do tipo da forma, e o primário dela é o mesmo da base em todas — a regra de
    // sempre (`tipoDaPedraDeEvolucao`) já dá a certa. O elo precisa estar AQUI, e não só na tabela das
    // ramificadas: a pedra da evolução (`pedraDeEvolucao`) exige `evolvesToId`. O Kyurem abre em dois
    // (White e Black) e está também em `EVOLUCOES_RAMIFICADAS`.
    483: { para: 8483, nivel: 3500 },
    // Dialga            → Dialga Origin
    484: { para: 8484, nivel: 3500 },
    // Palkia            → Palkia Origin
    492: { para: 8492, nivel: 3500 },
    // Shaymin           → Shaymin Sky
    641: { para: 8641, nivel: 8600 },
    // Tornadus          → Tornadus Therian
    642: { para: 8642, nivel: 8600 },
    // Thundurus         → Thundurus Therian
    645: { para: 8645, nivel: 8600 },
    // Landorus          → Landorus Therian
    646: { para: 8646, nivel: 8600 },
    // Kyurem            → White Kyurem (ou Black)
    681: { para: 8681, nivel: 21200 },
    // Aegislash         → Aegislash Blade
    720: { para: 8720, nivel: 21200 },
    // Hoopa             → Hoopa Unbound
    774: { para: 8774, nivel: 84650 },
    // Minior            → Minior Core
    888: { para: 8888, nivel: 35150 },
    // Zacian            → Zacian Crowned
    889: { para: 8889, nivel: 35150 },
    // Zamazenta         → Zamazenta Crowned
    905: { para: 8905, nivel: 35150 },
    // Enamorus          → Enamorus Therian
    964: { para: 8964, nivel: 53350 },
    // Palafin           → Palafin Hero
    // --- A leva de sprites de 30/10/2026: as formas novas, pela mesma régua (a pedra do tipo, o nível do topo
    // da região de quem não tem hunt), e a Bloodmoon Ursaluna, que o dono apontou solta — ela tem hunt em
    // Paldea, e o nível é o do marcador dela. Calyrex e Ogerpon abrem em mais de um destino e estão também
    // em `EVOLUCOES_RAMIFICADAS`; aqui fica o primeiro, que é o que a pedra lê.
    487: { para: 8487, nivel: 3500 },
    // Giratina          → Giratina Origin
    648: { para: 8648, nivel: 8600 },
    // Meloetta          → Meloetta Pirouette
    898: { para: 8898, nivel: 64950 },
    // Calyrex           → Calyrex Ice Rider (ou Shadow Rider)
    901: { para: 8901, nivel: 160300 },
    // Ursaluna          → Bloodmoon Ursaluna
    1017: { para: 9017, nivel: 160300 }
    // Ogerpon           → Ogerpon Wellspring (Hearthflame, Cornerstone)
  };
  function aplicarElosEvolutivos(lista, tabela) {
    const porId = new Map(lista.map((c) => [c.pokeId, c]));
    let ligados = 0;
    for (const [id, { para, nivel }] of Object.entries(tabela)) {
      const esp = porId.get(Number(id));
      if (!esp || esp.evolvesToId || !porId.has(para)) continue;
      esp.evolvesToId = para;
      esp.evolveLevel = nivel;
      ligados++;
    }
    return ligados;
  }
  function aplicarEvolucoesEntreGeracoes(lista) {
    return aplicarElosEvolutivos(lista, EVOLUCOES_ENTRE_GERACOES);
  }
  var NIVEIS_POR_ONDE_O_ALVO_APARECE = {
    // --- Alvo em SINNOH (dex 387–493)
    82: 1500,
    // Magneton   → Magnezone
    108: 1500,
    // Lickitung  → Lickilicky
    112: 3e3,
    // Rhydon     → Rhyperior
    114: 3e3,
    // Tangela    → Tangrowth
    125: 3e3,
    // Electabuzz → Electivire
    126: 3e3,
    // Magmar     → Magmortar
    176: 1500,
    // Togetic    → Togekiss
    190: 1250,
    // Aipom      → Ambipom
    193: 1500,
    // Yanma      → Yanmega
    198: 1500,
    // Murkrow    → Honchkrow
    200: 1250,
    // Misdreavus → Mismagius
    207: 1500,
    // Gligar     → Gliscor
    215: 1500,
    // Sneasel    → Weavile
    221: 1500,
    // Piloswine  → Mamoswine
    233: 1500,
    // Porygon2   → Porygon-Z
    299: 2e3,
    // Nosepass   → Probopass
    315: 3500,
    // Roselia    → Roserade
    356: 3e3,
    // Dusclops   → Dusknoir
    387: 1750,
    // Turtwig    → Grotle
    388: 3500,
    // Grotle     → Torterra
    390: 1750,
    // Chimchar   → Monferno
    391: 3500,
    // Monferno   → Infernape
    393: 1750,
    // Piplup     → Prinplup
    394: 3500,
    // Prinplup   → Empoleon
    396: 1750,
    // Starly     → Staravia
    397: 3500,
    // Staravia   → Staraptor
    399: 2e3,
    // Bidoof     → Bibarel
    401: 2e3,
    // Kricketot  → Kricketune
    403: 1750,
    // Shinx      → Luxio
    404: 3500,
    // Luxio      → Luxray
    408: 2e3,
    // Cranidos   → Rampardos
    410: 3e3,
    // Shieldon   → Bastiodon
    415: 3e3,
    // Combee     → Vespiquen
    418: 2e3,
    // Buizel     → Floatzel
    420: 2e3,
    // Cherubi    → Cherrim
    422: 2e3,
    // Shellos    → Gastrodon
    425: 2e3,
    // Drifloon   → Drifblim
    427: 3e3,
    // Buneary    → Lopunny
    431: 2e3,
    // Glameow    → Purugly
    434: 2e3,
    // Stunky     → Skuntank
    436: 2e3,
    // Bronzor    → Bronzong
    443: 1750,
    // Gible      → Gabite
    444: 3500,
    // Gabite     → Garchomp
    447: 2e3,
    // Riolu      → Lucario
    449: 2e3,
    // Hippopotas → Hippowdon
    451: 2e3,
    // Skorupi    → Drapion
    453: 2e3,
    // Croagunk   → Toxicroak
    456: 2e3,
    // Finneon    → Lumineon
    459: 2e3,
    // Snover     → Abomasnow
    489: 2e3,
    // Phione     → Manaphy
    13299: 2e3,
    // Nosepass   → Probopass
    13315: 3500,
    // Roselia    → Roserade
    13447: 2e3,
    // Riolu      → Lucario
    // --- Alvo em UNOVA (dex 494–649)
    495: 6400,
    // Snivy      → Servine
    496: 8600,
    // Servine    → Serperior
    498: 6400,
    // Tepig      → Pignite
    499: 8600,
    // Pignite    → Emboar
    501: 6400,
    // Oshawott   → Dewott
    502: 8600,
    // Dewott     → Samurott
    504: 6950,
    // Patrat     → Watchog
    507: 8600,
    // Herdier    → Stoutland
    509: 6950,
    // Purrloin   → Liepard
    511: 6950,
    // Pansage    → Simisage
    513: 6950,
    // Pansear    → Simisear
    515: 6950,
    // Panpour    → Simipour
    517: 6950,
    // Munna      → Musharna
    519: 6400,
    // Pidove     → Tranquill
    520: 8600,
    // Tranquill  → Unfezant
    522: 6950,
    // Blitzle    → Zebstrika
    524: 6400,
    // Roggenrola → Boldore
    525: 8600,
    // Boldore    → Gigalith
    527: 6950,
    // Woobat     → Swoobat
    529: 6950,
    // Drilbur    → Excadrill
    532: 6400,
    // Timburr    → Gurdurr
    533: 8600,
    // Gurdurr    → Conkeldurr
    535: 6400,
    // Tympole    → Palpitoad
    536: 8600,
    // Palpitoad  → Seismitoad
    540: 6400,
    // Sewaddle   → Swadloon
    541: 8600,
    // Swadloon   → Leavanny
    543: 6400,
    // Venipede   → Whirlipede
    544: 8600,
    // Whirlipede → Scolipede
    546: 6950,
    // Cottonee   → Whimsicott
    548: 6950,
    // Petilil    → Lilligant
    551: 6400,
    // Sandile    → Krokorok
    552: 8600,
    // Krokorok   → Krookodile
    557: 6950,
    // Dwebble    → Crustle
    559: 6950,
    // Scraggy    → Scrafty
    562: 6950,
    // Yamask     → Cofagrigus
    564: 6950,
    // Tirtouga   → Carracosta
    566: 8e3,
    // Archen     → Archeops
    568: 6950,
    // Trubbish   → Garbodor
    570: 6950,
    // Zorua      → Zoroark
    572: 6950,
    // Minccino   → Cinccino
    574: 6400,
    // Gothita    → Gothorita
    575: 8600,
    // Gothorita  → Gothitelle
    577: 6400,
    // Solosis    → Duosion
    578: 8600,
    // Duosion    → Reuniclus
    580: 6950,
    // Ducklett   → Swanna
    583: 8600,
    // Vanillish  → Vanilluxe
    585: 6950,
    // Deerling   → Sawsbuck
    588: 6950,
    // Karrablast → Escavalier
    590: 6950,
    // Foongus    → Amoonguss
    592: 6950,
    // Frillish   → Jellicent
    595: 6950,
    // Joltik     → Galvantula
    597: 6950,
    // Ferroseed  → Ferrothorn
    600: 8600,
    // Klang      → Klinklang
    602: 6400,
    // Tynamo     → Eelektrik
    603: 8600,
    // Eelektrik  → Eelektross
    607: 6400,
    // Litwick    → Lampent
    608: 8600,
    // Lampent    → Chandelure
    610: 6400,
    // Axew       → Fraxure
    611: 8600,
    // Fraxure    → Haxorus
    613: 6950,
    // Cubchoo    → Beartic
    616: 6950,
    // Shelmet    → Accelgor
    619: 6950,
    // Mienfoo    → Mienshao
    622: 6950,
    // Golett     → Golurk
    624: 6400,
    // Pawniard   → Bisharp
    627: 6950,
    // Rufflet    → Braviary
    629: 6950,
    // Vullaby    → Mandibuzz
    633: 6400,
    // Deino      → Zweilous
    634: 8600,
    // Zweilous   → Hydreigon
    636: 6950,
    // Larvesta   → Volcarona
    // --- Alvo em KALOS (dex 650–721)
    651: 21200,
    // Quilladin   → Chesnaught
    654: 21200,
    // Braixen     → Delphox
    659: 16e3,
    // Bunnelby    → Diggersby
    661: 14350,
    // Fletchling  → Fletchinder
    662: 21200,
    // Fletchinder → Talonflame
    664: 14350,
    // Scatterbug  → Spewpa
    665: 21200,
    // Spewpa      → Vivillon
    667: 16e3,
    // Litleo      → Pyroar
    669: 14350,
    // Flabebe     → Floette
    670: 21200,
    // Floette     → Florges
    674: 2e4,
    // Pancham     → Pangoro
    679: 14350,
    // Honedge     → Doublade
    680: 1e3,
    // Doublade    → Aegislash
    682: 16e3,
    // Spritzee    → Aromatisse
    684: 16e3,
    // Swirlix     → Slurpuff
    686: 16e3,
    // Inkay       → Malamar
    688: 16e3,
    // Binacle     → Barbaracle
    690: 2e4,
    // Skrelp      → Dragalge
    692: 16e3,
    // Clauncher   → Clawitzer
    694: 16e3,
    // Helioptile  → Heliolisk
    698: 16e3,
    // Amaura      → Aurorus
    704: 14350,
    // Goomy       → Sliggoo
    705: 21200,
    // Sliggoo     → Goodra
    708: 16e3,
    // Phantump    → Trevenant
    712: 16e3,
    // Bergmite    → Avalugg
    714: 16e3,
    // Noibat      → Noivern
    // --- Alvo em ALOLA (dex 722–809)
    722: 32e3,
    // Rowlet    → Dartrix
    725: 32e3,
    // Litten    → Torracat
    726: 43100,
    // Torracat  → Incineroar
    728: 32e3,
    // Popplio   → Brionne
    729: 43100,
    // Brionne   → Primarina
    731: 32e3,
    // Pikipek   → Trumbeak
    732: 43100,
    // Trumbeak  → Toucannon
    734: 34650,
    // Yungoos   → Gumshoos
    736: 32e3,
    // Grubbin   → Charjabug
    737: 43100,
    // Charjabug → Vikavolt
    742: 34650,
    // Cutiefly  → Ribombee
    747: 34650,
    // Mareanie  → Toxapex
    749: 34650,
    // Mudbray   → Mudsdale
    751: 34650,
    // Dewpider  → Araquanid
    755: 34650,
    // Morelull  → Shiinotic
    757: 34650,
    // Salandit  → Salazzle
    759: 34650,
    // Stufful   → Bewear
    761: 32e3,
    // Bounsweet → Steenee
    762: 43100,
    // Steenee   → Tsareena
    767: 34650,
    // Wimpod    → Golisopod
    782: 32e3,
    // Jangmo-o  → Hakamo-o
    783: 43100,
    // Hakamo-o  → Kommo-o
    // --- BEBÊS — base de geração posterior ao alvo, então a hunt dele fica abaixo da do bebê
    172: 60,
    // Pichu     → Pikachu
    173: 40,
    // Cleffa    → Clefairy
    174: 40,
    // Igglybuff → Jigglypuff
    238: 100,
    // Smoochum  → Jynx
    239: 100,
    // Elekid    → Electabuzz
    240: 100,
    // Magby     → Magmar
    298: 20,
    // Azurill   → Marill
    360: 100,
    // Wynaut    → Wobbuffet
    406: 650,
    // Budew     → Roselia
    433: 700,
    // Chingling → Chimecho
    439: 80,
    // Mime Jr.  → Mr. Mime
    446: 100,
    // Munchlax  → Snorlax
    458: 100,
    // Mantyke   → Mantine
    13298: 20,
    // Azurill   → Marill
    13360: 100,
    // Wynaut    → Wobbuffet
    // --- Alvo FORA de Sinnoh–Alola — os cinco que a tabela já trazia, intactos
    113: 1125,
    // Chansey → Blissey
    137: 1125,
    // Porygon → Porygon2
    349: 1125,
    // Feebas  → Milotic
    375: 1250,
    // Metang  → Metagross
    803: 7500
    // Poipole → Naganadel
  };
  function corrigirNiveisDeEvolucao(lista) {
    let corrigidos = 0;
    for (const c of lista) {
      const nivel = NIVEIS_POR_ONDE_O_ALVO_APARECE[c.pokeId];
      if (!nivel || !c.evolvesToId || c.evolveLevel === nivel) continue;
      c.evolveLevel = nivel;
      corrigidos++;
    }
    return corrigidos;
  }
  function normalizarEvolveLevel600(lista) {
    for (const c of lista) {
      if (c.evolveLevel === 600) c.evolveLevel = 100;
    }
  }
  function aplicarCadeiasTruncadas(lista) {
    return aplicarElosEvolutivos(lista, CADEIAS_TRUNCADAS);
  }

  // tests/.cache/shared/recompensa-hunt.mjs
  var recompensa_hunt_exports = {};
  __export(recompensa_hunt_exports, {
    EXPOENTE_RENDA: () => EXPOENTE_RENDA,
    FRACAO_BONUS_LOOT: () => FRACAO_BONUS_LOOT,
    alvoRendaPorKill: () => alvoRendaPorKill,
    ouroDoAbate: () => ouroDoAbate,
    ouroDoLootPorKill: () => ouroDoLootPorKill,
    ouroDoNivel: () => ouroDoNivel,
    ouroPorDerrotaHunt: () => ouroPorDerrotaHunt,
    xpDoNivel: () => xpDoNivel,
    xpPorDerrota: () => xpPorDerrota
  });

  // tests/.cache/shared/sell-value.mjs
  var NIVEL_ANCORA = 150;
  var OURO_ANCORA = 12e3;
  var XP_ANCORA = 0.6 * NIVEL_ANCORA ** 2;
  var xpDoNivel = (n) => n <= NIVEL_ANCORA ? Math.floor(6 * n * n / 10) + 8 : Math.round(XP_ANCORA * (n / NIVEL_ANCORA) ** 1.25);
  var ouroDoNivel = (n) => Math.round(OURO_ANCORA * (n / NIVEL_ANCORA) ** 0.6);
  var OURO_KILL_NIVEL_1 = 36;
  var OURO_KILL_ANCORA_NIVEL = 5e3;
  var OURO_KILL_ANCORA = 6e3;
  var EXPOENTE_OURO_KILL = Math.log(OURO_KILL_ANCORA / OURO_KILL_NIVEL_1) / Math.log(OURO_KILL_ANCORA_NIVEL);
  var NIVEL_TETO_OURO = 25e3;
  var TETO_OURO_KILL = Math.round(OURO_KILL_NIVEL_1 * NIVEL_TETO_OURO ** EXPOENTE_OURO_KILL);
  var ouroPorKillDoNivel = (n) => Math.min(
    TETO_OURO_KILL,
    Math.max(1, Math.round(OURO_KILL_NIVEL_1 * Math.max(1, Number(n) || 1) ** EXPOENTE_OURO_KILL))
  );
  var NIVEL_HUNT_TETO_SELL_POKEMON = 2500;
  var TETO_SELL_POKEMON = ouroDoNivel(NIVEL_HUNT_TETO_SELL_POKEMON);

  // tests/.cache/shared/recompensa-hunt.mjs
  function xpPorDerrota(hunt, nivelMob, especie) {
    return xpDoNivel(nivelMob);
  }
  var FRACAO_BONUS_LOOT = 0.15;
  var EXPOENTE_RENDA = 0.3;
  function alvoRendaPorKill(nivel) {
    const n = Math.max(1, Number(nivel) || 1);
    const base = n <= NIVEL_TETO_OURO ? ouroPorKillDoNivel(n) : TETO_OURO_KILL * (n / NIVEL_TETO_OURO) ** EXPOENTE_RENDA;
    return Math.round(base * (1 + FRACAO_BONUS_LOOT));
  }
  function ouroDoLootPorKill(nivel, bonusLootPct2 = 0) {
    const base = Math.max(0, alvoRendaPorKill(nivel) - ouroPorKillDoNivel(nivel));
    const bonus = Math.max(0, Number(bonusLootPct2) || 0);
    return Math.round(base * (1 + bonus / 100));
  }
  function ouroDoAbate(nivelMob, bonusLootPct2 = 0) {
    const base = ouroPorKillDoNivel(nivelMob);
    const loot = ouroDoLootPorKill(nivelMob, bonusLootPct2);
    return { base, loot, total: base + loot };
  }
  function ouroPorDerrotaHunt(hunt, nivelMob, especie, bonusLootPct2 = 0) {
    return ouroDoAbate(nivelMob, bonusLootPct2).total;
  }

  // tests/.cache/shared/ajustes-especie-espelho.mjs
  var ajustes_especie_espelho_exports = {};
  __export(ajustes_especie_espelho_exports, {
    AJUSTE_HUNT_LEVEL: () => AJUSTE_HUNT_LEVEL,
    AJUSTE_TIPOS: () => AJUSTE_TIPOS,
    aplicarAjustesHuntLevel: () => aplicarAjustesHuntLevel,
    aplicarAjustesTipos: () => aplicarAjustesTipos
  });
  var AJUSTE_HUNT_LEVEL = {
    112: 100,
    // Rhydon — hunt Kanto nv 100; espelho deixou 80 (degrau do Rhyhorn)
    219: 100,
    // Magcargo — mesmo bug (degrau do Slugma)
    13447: 1e3,
    // Riolu
    13448: 1250,
    // Lucario
    297: 700,
    // Hariyama — acima do Makuhita (550)
    13297: 700,
    // Hariyama (clone Hoenn)
    // Smeargle é de Gen 2 e mora em Johto (`hunts-johto-novos.json`); o espelho deixou
    // huntLevel 1 com `priceNpc` de 1 milhão, que é lixo de importação. 30 é o degrau dos
    // vizinhos de ilha dele — Dunsparce, Snubbull, Teddiursa —, e é daqui (e não do degrau
    // de `hunts-sinnoh.json`, que o cliente não lê) para as duas pontas verem o mesmo XP.
    // Desde 17/10/2026 (Johto no Nv 200) a hunt dele e a dos vizinhos estão no 290: o 30 aqui é o
    // degrau na escala do espelho, e o valor sobe à curva da hunt em `shared/valor-cadeia.mjs`.
    235: 30,
    // Smeargle
    // Ditto é de Kanto e mora em `hunts-kanto-novos.json` desde 18/10/2026 (pedido do dono), no
    // Nv 50 do Farfetch'd. O espelho deixou huntLevel 1 e `priceNpc` 0 — enquanto ele estava em
    // `hunts-sinnoh.json` o degrau de 1.000 de lá cobria isso no servidor, e fora dele a venda
    // caía para 594. Como no Smeargle: o nível da hunt, para as duas pontas verem o mesmo valor.
    132: 50,
    // Ditto
    // Weavile e Togekiss existem DUAS vezes no catálogo (o espelho e o nosso), e o espelho traz Sneasel → Weavile e
    // Togetic → Togekiss. Montada com ele junto (`aplicarEscalaHuntLevel` no content.mjs), a escada de Sinnoh punha os
    // dois no topo, 3000, só no servidor; o cliente, que não monta a escada, mostrava o 1500 do nosso catálogo — o nível
    // das hunts deles. O card do Market → Vender mostrava um preço e a venda pagava 1,52× (relato do dono, 21/10/2026: um
    // Weavile Nv 100 Q 1,45 mostrava 207.812 e pagou 314.983). Ele escolheu o do card: o nível da hunt.
    461: 1500,
    // Weavile — hunt weavile_sinnoh nv 1500
    468: 1500
    // Togekiss — hunt togekiss_sinnoh nv 1500
  };
  function aplicarAjustesHuntLevel(obter) {
    for (const [id, nivel] of Object.entries(AJUSTE_HUNT_LEVEL)) {
      const esp = obter(Number(id));
      if (!esp) continue;
      esp.huntLevel = nivel;
      if (nivel <= NIVEL_ANCORA) {
        esp.experience = xpDoNivel(nivel);
        const ouro = ouroDoNivel(nivel);
        esp.sellValue = ouro;
        esp.priceNpc = ouro;
      }
    }
  }
  var AJUSTE_TIPOS = {
    448: ["FIGHTING", "STEEL"],
    // Lucario
    13448: ["FIGHTING", "STEEL"]
    // Lucario (clone de Orre)
  };
  function aplicarAjustesTipos(obter) {
    for (const [id, [type1, type2]] of Object.entries(AJUSTE_TIPOS)) {
      for (const esp of obter(Number(id)) ?? []) {
        esp.type1 = type1;
        esp.type2 = type2 ?? null;
      }
    }
  }

  // tests/.cache/raiz/hunt-analyser.mjs
  var hunt_analyser_exports = {};
  __export(hunt_analyser_exports, {
    CD_GOLPE_MS: () => CD_GOLPE_MS,
    CD_MOVIMENTO_MEDIO_MS: () => CD_MOVIMENTO_MEDIO_MS,
    MAX_MOBS: () => MAX_MOBS,
    MS_DESLOC_MS: () => MS_DESLOC_MS,
    MS_MINIMO_RITMO: () => MS_MINIMO_RITMO,
    MS_ONDA: () => MS_ONDA,
    TIPOS_ELEMENTAIS: () => TIPOS_ELEMENTAIS,
    VIP_MULT_XP: () => VIP_MULT_XP,
    analisarHunt: () => analisarHunt,
    bonusLootPct: () => bonusLootPct,
    efetividade: () => efetividade2,
    especiePassaFiltroMatchup: () => especiePassaFiltroMatchup,
    killsPorHora: () => killsPorHora,
    listaDropsEspecie: () => listaDropsEspecie,
    matchupDefensivo: () => matchupDefensivo,
    matchupTiposHunt: () => matchupTiposHunt,
    mediaPorKill: () => mediaPorKill,
    melhorEfetividadeContra: () => melhorEfetividadeContra,
    multXpPokemon: () => multXpPokemon,
    multXpTreinador: () => multXpTreinador,
    ritmoDaSessao: () => ritmoDaSessao
  });

  // tests/.cache/shared/tipo-efetividade.mjs
  var DIAGONAL_DE_TIPOS = Object.freeze({
    FIRE: 0.5,
    WATER: 0.5,
    ELECTRIC: 0.5,
    GRASS: 0.5,
    ICE: 0.5,
    POISON: 0.5,
    PSYCHIC: 0.5,
    DARK: 0.5,
    STEEL: 0.5,
    GHOST: 2,
    DRAGON: 2
  });
  function multAtaqueContraDefesa(atq, def, linha) {
    const a = String(atq).toUpperCase();
    const d = String(def).toUpperCase();
    if (d === a) return DIAGONAL_DE_TIPOS[a] ?? 1;
    return linha[d] ?? 1;
  }
  function efetividade(atq, def1, def2, tabela) {
    const linha = tabela[String(atq).toUpperCase()] ?? {};
    let v = multAtaqueContraDefesa(atq, def1, linha);
    if (def2 && def2 !== def1) v *= multAtaqueContraDefesa(atq, def2, linha);
    return v;
  }

  // tests/.cache/shared/guild-boost.mjs
  var BOOST_GUILD = Object.freeze({
    /** Quanto o boost soma no XP do treinador E no do pokemon. */
    pct: 10,
    dias: 7,
    /** A arte inteira (48×48: a aba Boost, o diálogo, a comemoração) e a pequena (32×32: o que sai a 16-20 px). */
    icone: "/img/itens/boost-guild.png",
    iconeMini: "/img/itens/boost-guild-32.png"
  });
  var BOOST_GUILD_MS = BOOST_GUILD.dias * 24 * 36e5;
  var BOOST_GUILD_MULT = 1 + BOOST_GUILD.pct / 100;
  var BANCO_GUILD = Object.freeze({
    /** Quanto o banco junta para ligar (ou estender) o boost: 7 dias. Era 1.000 até 08/10/2026. */
    meta: 500,
    /** O `ref` das linhas do ledger: a doação (`loja`) e a devolução da guild apagada (`loja_estorno`). */
    ref: "guild_banco",
    /** Os atalhos de doação da tela, além do "Completar". */
    atalhos: [10, 50, 100, 250],
    /** Quantas linhas o histórico do banco traz (doações e bancos cheios, os mais recentes). */
    historico: 25,
    /**
     * Quantas doações o LOG COMPLETO (dono e sub-dono, "Ver tudo" na aba) traz no máximo — as mais
     * recentes. O teto é do socket, que leva a resposta inteira de uma vez: 5.000 linhas são ~200 kB,
     * e uma guild que enche um banco por semana leva anos para chegar nelas. A soma por membro é
     * sempre da vida inteira, com teto ou sem (ver `logBancoDaGuild`).
     */
    logMax: 5e3
  });
  var boostGuildAtivo = (ate, agora) => (Number(ate) || 0) > agora;
  var multBoostGuild = (ate, agora) => boostGuildAtivo(ate, agora) ? BOOST_GUILD_MULT : 1;

  // tests/.cache/raiz/hunt-analyser.mjs
  var MS_ONDA = 2600;
  var MAX_MOBS = 16;
  var CD_GOLPE_MS = 900;
  var MS_DESLOC_MS = 600;
  var CD_MOVIMENTO_MEDIO_MS = 2e3;
  var VIP_MULT_XP = 1.5;
  var TIPOS_ELEMENTAIS = [
    "NORMAL",
    "FIRE",
    "WATER",
    "GRASS",
    "ELECTRIC",
    "ICE",
    "FIGHTING",
    "POISON",
    "GROUND",
    "FLYING",
    "PSYCHIC",
    "BUG",
    "ROCK",
    "GHOST",
    "DRAGON",
    "DARK",
    "STEEL",
    "FAIRY"
  ];
  function efetividade2(atq, def1, def2, tabela) {
    return efetividade(atq, def1, def2, tabela);
  }
  function multBoost(loja, chave, agora, tiposBoost) {
    const ate = loja?.boosts?.[chave] ?? 0;
    if (agora >= ate) return 1;
    return tiposBoost.find((b) => b.chave === chave)?.mult ?? 1;
  }
  var multGuild = (p) => 1 + (Number(p?.guildBonusPct) || 0) / 100;
  var multBoostDaGuild = (p, agora) => multBoostGuild(p?.guild?.boostAte, agora);
  var vipAtivo = (loja, agora) => agora < (loja?.vipAte ?? 0);
  function multXpTreinador(p, agora, tiposBoost) {
    const l = p.loja ?? {};
    return multBoost(l, "xp", agora, tiposBoost) * (vipAtivo(l, agora) ? VIP_MULT_XP : 1) * multGuild(p) * multBoostDaGuild(p, agora);
  }
  function multXpPokemon(p, agora, tiposBoost) {
    const l = p.loja ?? {};
    return multBoost(l, "pokexp", agora, tiposBoost) * (vipAtivo(l, agora) ? VIP_MULT_XP : 1) * multGuild(p) * multBoostDaGuild(p, agora);
  }
  function bonusLootPct(p, agora, tiposBoost, farmEventoPct = 0) {
    const boost = (multBoost(p?.loja ?? {}, "loot", agora, tiposBoost) - 1) * 100;
    return boost + (Number(p?.guildBonusPct) || 0) + (Number(farmEventoPct) || 0);
  }
  function mediaPorKill(hunt, especies, bonusLoot = 0) {
    const total = hunt.especies.reduce((s, e) => s + e.pontos, 0) || 1;
    let xp = 0;
    let gold = 0;
    for (const e of hunt.especies) {
      const esp = especies.get(e.pokeId);
      const peso = e.pontos / total;
      const base = xpPorDerrota(hunt, hunt.nivel, esp);
      xp += base * peso;
      gold += ouroPorDerrotaHunt(hunt, hunt.nivel, esp, bonusLoot) * peso;
    }
    return { xp, gold };
  }
  function killsPorHora(hunt) {
    const pontos = hunt.especies.reduce((s, e) => s + e.pontos, 0) || 1;
    const porOnda = Math.min(MAX_MOBS, pontos);
    const msPorKill = CD_GOLPE_MS + MS_DESLOC_MS + CD_MOVIMENTO_MEDIO_MS;
    const segPorOnda = (porOnda * msPorKill + MS_ONDA) / 1e3;
    return porOnda / segPorOnda * 3600;
  }
  function melhorEfetividadeContra(tiposAtaque, defType1, defType2, tabela) {
    let melhor = 0;
    for (const atk of tiposAtaque) {
      const v = efetividade2(atk, defType1, defType2, tabela);
      if (v > melhor) melhor = v;
    }
    return melhor;
  }
  function especiePassaFiltroMatchup(esp, tiposSel, modo, tabela) {
    if (!esp || !tiposSel?.length || !modo) return false;
    const mult = melhorEfetividadeContra(tiposSel, esp.type1, esp.type2, tabela);
    if (modo === "fraco") return mult >= 2;
    if (modo === "forte") return mult <= 0.5;
    return false;
  }
  function matchupDefensivo(tiposDefensor, tabela) {
    const fracos = [];
    const fortes = [];
    const d1 = tiposDefensor[0];
    const d2 = tiposDefensor[1];
    if (!d1) return { fracos, fortes };
    for (const atk of TIPOS_ELEMENTAIS) {
      const mult = efetividade2(atk, d1, d2 ?? null, tabela);
      if (mult >= 2) fracos.push({ tipo: atk, mult });
      else if (mult <= 0.5) fortes.push({ tipo: atk, mult });
    }
    return { fracos, fortes };
  }
  function matchupTiposHunt(tiposAtaque, tabela) {
    const fortes = [];
    const fracos = [];
    for (const def of TIPOS_ELEMENTAIS) {
      let melhor = 0;
      for (const atk of tiposAtaque) {
        const v = efetividade2(atk, def, null, tabela);
        if (v > melhor) melhor = v;
      }
      if (melhor >= 2) fortes.push({ tipo: def, mult: melhor });
      else if (melhor <= 0.5) fracos.push({ tipo: def, mult: melhor });
    }
    return { fortes, fracos };
  }
  function listaDropsEspecie(especie) {
    return (especie?.loot ?? []).filter((l) => l.chance > 0).map((l) => ({
      name: l.name,
      pct: l.chance / 1e3,
      minCount: l.minCount,
      maxCount: l.maxCount
    })).sort((a, b) => b.pct - a.pct);
  }
  var MS_MINIMO_RITMO = 6e4;
  function ritmoDaSessao(sessao, agora = Date.now()) {
    if (!sessao?.inicio) return null;
    const horas = Math.max(agora - sessao.inicio, MS_MINIMO_RITMO) / 36e5;
    return {
      // `xpH` é a SOMA das duas XPs — o Hunt Analyser ainda mostra assim. O Pocket mostra as duas
      // separadas: somadas, elas davam o dobro do que a barra de cada uma de fato anda por hora.
      xpH: Math.round(((sessao.xpTreinador || 0) + (sessao.xpPokemon || 0)) / horas),
      xpTreinadorH: Math.round((sessao.xpTreinador || 0) / horas),
      xpPokemonH: Math.round((sessao.xpPokemon || 0) / horas),
      ouroH: Math.round((sessao.gold || 0) / horas),
      abatesH: Math.round((sessao.abates || 0) / horas),
      capturasH: Math.round((sessao.capturas || 0) / horas)
    };
  }
  function analisarHunt(hunt, ctx, tiposAtaque = []) {
    const { especies, tabelaTipos, eu, tiposBoost, farmEventoPct = 0 } = ctx;
    const agora = Date.now();
    const media = mediaPorKill(hunt, especies, bonusLootPct(eu, agora, tiposBoost, farmEventoPct));
    const kph = killsPorHora(hunt);
    const multTr = multXpTreinador(eu, agora, tiposBoost);
    const multPk = multXpPokemon(eu, agora, tiposBoost);
    const { fracos, fortes } = matchupDefensivo(tiposAtaque, tabelaTipos);
    return {
      hunt,
      xpTreinadorH: Math.round(media.xp * kph * multTr),
      xpPokemonH: Math.round(media.xp * kph * multPk),
      goldH: Math.round(media.gold * kph),
      killsH: Math.round(kph),
      tiposFracos: fracos,
      tiposResiste: fortes
    };
  }

  // tests/.cache/shared/ginasios.mjs
  var ginasios_exports = {};
  __export(ginasios_exports, {
    ARENA_EXPO_GINASIO: () => ARENA_EXPO_GINASIO,
    ARENA_EXPO_GUERRA: () => ARENA_EXPO_GUERRA,
    ARENA_NIVEL_CHEIO: () => ARENA_NIVEL_CHEIO,
    GINASIO_BUFF_MULT: () => GINASIO_BUFF_MULT,
    GINASIO_BUFF_PCT: () => GINASIO_BUFF_PCT,
    GINASIO_BUFF_PODIO_PCT: () => GINASIO_BUFF_PODIO_PCT,
    GINASIO_DESAFIO_COOLDOWN_MS: () => GINASIO_DESAFIO_COOLDOWN_MS,
    GINASIO_ESCALA_FORCA: () => GINASIO_ESCALA_FORCA,
    GINASIO_NIVEL_MIN: () => GINASIO_NIVEL_MIN,
    GINASIO_TIME_MAX: () => GINASIO_TIME_MAX,
    SLUG_ARENA_GINASIO: () => SLUG_ARENA_GINASIO,
    TIPOS_GINASIO: () => TIPOS_GINASIO,
    VARIACAO_DANO_MAX: () => VARIACAO_DANO_MAX,
    VARIACAO_DANO_MIN: () => VARIACAO_DANO_MIN,
    buffGinasioDoPokemon: () => buffGinasioDoPokemon,
    compararDuelo1v1: () => compararDuelo1v1,
    ginasioDeCadaPokemon: () => ginasioDeCadaPokemon,
    hpDeCombateGinasio: () => hpDeCombateGinasio,
    iconeDoGinasio: () => iconeDoGinasio,
    idsQueContamNoGinasio: () => idsQueContamNoGinasio,
    melhorGolpeDoGinasio: () => melhorGolpeDoGinasio,
    metricasDeCombate: () => metricasDeCombate,
    motivoForaDoGinasio: () => motivoForaDoGinasio,
    multBuffGinasio: () => multBuffGinasio,
    multDoLugarGinasio: () => multDoLugarGinasio,
    nivelDeArena: () => nivelDeArena,
    nivelNaGuerra: () => nivelNaGuerra,
    nivelNoGinasio: () => nivelNoGinasio,
    outroGinasioDoPokemon: () => outroGinasioDoPokemon,
    pctDoLugarGinasio: () => pctDoLugarGinasio,
    poderDoTimeGinasio: () => poderDoTimeGinasio,
    poderNoGinasio: () => poderNoGinasio,
    poderNoNivel: () => poderNoNivel,
    pokemonDoTipo: () => pokemonDoTipo,
    pokemonEmGinasioDosTimes: () => pokemonEmGinasioDosTimes,
    ratingDeCombate: () => ratingDeCombate,
    segundoETerceiro: () => segundoETerceiro,
    statsNoGinasio: () => statsNoGinasio,
    statsNoNivel: () => statsNoNivel,
    timeValidoDoGinasio: () => timeValidoDoGinasio,
    timesQueContam: () => timesQueContam,
    tipoDeGinasio: () => tipoDeGinasio,
    validarTimeGinasio: () => validarTimeGinasio
  });

  // tests/.cache/shared/refino-stats.mjs
  var STATS_REFINAVEIS = ["hp", "atk", "def", "spAtk", "spDef"];
  var REFINO_TETO_TECNICO = 9999;
  var REFINO_PASSO = 1;
  var inteiro = (v) => {
    const n = Math.floor(Number(v) || 0);
    return n > 0 ? Math.min(REFINO_TETO_TECNICO, n) : 0;
  };
  function normalizarRefino(bruto) {
    const cru = typeof bruto === "string" ? seguroJson(bruto) : bruto;
    const out = {};
    for (const k of STATS_REFINAVEIS) out[k] = inteiro(cru?.[k]);
    return out;
  }
  function seguroJson(txt) {
    try {
      return JSON.parse(txt);
    } catch {
      return null;
    }
  }
  function basesComRefino(especie, bonus) {
    const b = normalizarRefino(bonus);
    return {
      hp: (especie?.baseHp ?? 0) + b.hp * REFINO_PASSO,
      atk: (especie?.baseAtk ?? 0) + b.atk * REFINO_PASSO,
      def: (especie?.baseDef ?? 0) + b.def * REFINO_PASSO,
      spAtk: (especie?.baseSpAtk ?? 0) + b.spAtk * REFINO_PASSO,
      spDef: (especie?.baseSpDef ?? 0) + b.spDef * REFINO_PASSO,
      speed: especie?.baseSpeed ?? 0
    };
  }

  // tests/.cache/shared/formas.mjs
  var FORMAS = [
    { pokeId: 4019, dex: 19, nome: "Alolan Rattata", grupo: "alola", looktype: 64019, looktypeShiny: 74019 },
    { pokeId: 4020, dex: 20, nome: "Alolan Raticate", grupo: "alola", looktype: 64020, looktypeShiny: 74020 },
    { pokeId: 4026, dex: 26, nome: "Alolan Raichu", grupo: "alola", looktype: 64026, looktypeShiny: 74026 },
    { pokeId: 4027, dex: 27, nome: "Alolan Sandshrew", grupo: "alola", looktype: 64027, looktypeShiny: 74027 },
    { pokeId: 4028, dex: 28, nome: "Alolan Sandslash", grupo: "alola", looktype: 64028, looktypeShiny: 74028 },
    { pokeId: 4037, dex: 37, nome: "Alolan Vulpix", grupo: "alola", looktype: 64037, looktypeShiny: 74037 },
    { pokeId: 4038, dex: 38, nome: "Alolan Ninetales", grupo: "alola", looktype: 64038, looktypeShiny: 74038 },
    { pokeId: 4050, dex: 50, nome: "Alolan Diglett", grupo: "alola", looktype: 64050, looktypeShiny: 74050 },
    { pokeId: 4051, dex: 51, nome: "Alolan Dugtrio", grupo: "alola", looktype: 64051, looktypeShiny: 74051 },
    { pokeId: 4052, dex: 52, nome: "Alolan Meowth", grupo: "alola", looktype: 64052, looktypeShiny: 74052 },
    { pokeId: 5052, dex: 52, nome: "Galarian Meowth", grupo: "galar", looktype: 65052, looktypeShiny: 75052 },
    { pokeId: 4053, dex: 53, nome: "Alolan Persian", grupo: "alola", looktype: 64053, looktypeShiny: 74053 },
    { pokeId: 6058, dex: 58, nome: "Hisuian Growlithe", grupo: "hisui", looktype: 66058, looktypeShiny: 76058 },
    { pokeId: 6059, dex: 59, nome: "Hisuian Arcanine", grupo: "hisui", looktype: 66059, looktypeShiny: 76059 },
    { pokeId: 4074, dex: 74, nome: "Alolan Geodude", grupo: "alola", looktype: 64074, looktypeShiny: 74074 },
    { pokeId: 4075, dex: 75, nome: "Alolan Graveler", grupo: "alola", looktype: 64075, looktypeShiny: 74075 },
    { pokeId: 5077, dex: 77, nome: "Galarian Ponyta", grupo: "galar", looktype: 65077, looktypeShiny: 75077 },
    { pokeId: 5078, dex: 78, nome: "Galarian Rapidash", grupo: "galar", looktype: 65078, looktypeShiny: 75078 },
    { pokeId: 5079, dex: 79, nome: "Galarian Slowpoke", grupo: "galar", looktype: 65079, looktypeShiny: 75079 },
    { pokeId: 5080, dex: 80, nome: "Galarian Slowbro", grupo: "galar", looktype: 65080, looktypeShiny: 75080 },
    { pokeId: 5083, dex: 83, nome: "Galarian Farfetch'd", grupo: "galar", looktype: 65083, looktypeShiny: 75083 },
    { pokeId: 4088, dex: 88, nome: "Alolan Grimer", grupo: "alola", looktype: 64088, looktypeShiny: 74088 },
    { pokeId: 4089, dex: 89, nome: "Alolan Muk", grupo: "alola", looktype: 64089, looktypeShiny: 74089 },
    { pokeId: 6100, dex: 100, nome: "Hisuian Voltorb", grupo: "hisui", looktype: 66100, looktypeShiny: 76100 },
    { pokeId: 6101, dex: 101, nome: "Hisuian Electrode", grupo: "hisui", looktype: 66101, looktypeShiny: 76101 },
    { pokeId: 4103, dex: 103, nome: "Alolan Exeggutor", grupo: "alola", looktype: 64103, looktypeShiny: 74103 },
    { pokeId: 4105, dex: 105, nome: "Alolan Marowak", grupo: "alola", looktype: 64105, looktypeShiny: 74105 },
    { pokeId: 5110, dex: 110, nome: "Galarian Weezing", grupo: "galar", looktype: 65110, looktypeShiny: 75110 },
    { pokeId: 5122, dex: 122, nome: "Galarian Mr. Mime", grupo: "galar", looktype: 65122, looktypeShiny: 75122 },
    { pokeId: 7128, dex: 128, nome: "Paldean Tauros", grupo: "paldea", looktype: 67128, looktypeShiny: 77128 },
    { pokeId: 8128, dex: 128, nome: "Paldean Tauros Blaze", grupo: "forma", looktype: 68128, looktypeShiny: 78128 },
    { pokeId: 9128, dex: 128, nome: "Paldean Tauros Aqua", grupo: "forma", looktype: 69128, looktypeShiny: 79128 },
    { pokeId: 5144, dex: 144, nome: "Galarian Articuno", grupo: "galar", looktype: 65144, looktypeShiny: 75144 },
    { pokeId: 5145, dex: 145, nome: "Galarian Zapdos", grupo: "galar", looktype: 65145, looktypeShiny: 75145 },
    { pokeId: 5146, dex: 146, nome: "Galarian Moltres", grupo: "galar", looktype: 65146, looktypeShiny: 75146 },
    { pokeId: 6157, dex: 157, nome: "Hisuian Typhlosion", grupo: "hisui", looktype: 66157, looktypeShiny: 76157 },
    { pokeId: 7194, dex: 194, nome: "Paldean Wooper", grupo: "paldea", looktype: 67194, looktypeShiny: 77194 },
    { pokeId: 5199, dex: 199, nome: "Galarian Slowking", grupo: "galar", looktype: 65199, looktypeShiny: 75199 },
    { pokeId: 6211, dex: 211, nome: "Hisuian Qwilfish", grupo: "hisui", looktype: 66211, looktypeShiny: 76211 },
    { pokeId: 6215, dex: 215, nome: "Hisuian Sneasel", grupo: "hisui", looktype: 66215, looktypeShiny: 76215 },
    { pokeId: 5222, dex: 222, nome: "Galarian Corsola", grupo: "galar", looktype: 65222, looktypeShiny: 75222 },
    { pokeId: 5263, dex: 263, nome: "Galarian Zigzagoon", grupo: "galar", looktype: 65263, looktypeShiny: 75263 },
    { pokeId: 5264, dex: 264, nome: "Galarian Linoone", grupo: "galar", looktype: 65264, looktypeShiny: 75264 },
    { pokeId: 8351, dex: 351, nome: "Castform Rainy", grupo: "forma", looktype: 68351, looktypeShiny: 78351 },
    { pokeId: 9351, dex: 351, nome: "Castform Snowy", grupo: "forma", looktype: 69351, looktypeShiny: 79351 },
    { pokeId: 11351, dex: 351, nome: "Castform Sunny", grupo: "forma", looktype: 71351, looktypeShiny: 81351 },
    { pokeId: 8386, dex: 386, nome: "Deoxys Attack", grupo: "forma", looktype: 68386, looktypeShiny: 78386 },
    { pokeId: 9386, dex: 386, nome: "Deoxys Defense", grupo: "forma", looktype: 69386, looktypeShiny: 79386 },
    { pokeId: 11386, dex: 386, nome: "Deoxys Speed", grupo: "forma", looktype: 71386, looktypeShiny: 81386 },
    { pokeId: 8413, dex: 413, nome: "Wormadam Sandy", grupo: "forma", looktype: 68413, looktypeShiny: 78413 },
    { pokeId: 9413, dex: 413, nome: "Wormadam Trash", grupo: "forma", looktype: 69413, looktypeShiny: 79413 },
    { pokeId: 1479, dex: 479, nome: "Mow Rotom", grupo: "forma", looktype: 61479, looktypeShiny: 91479 },
    { pokeId: 8479, dex: 479, nome: "Heat Rotom", grupo: "forma", looktype: 68479, looktypeShiny: 78479 },
    { pokeId: 9479, dex: 479, nome: "Wash Rotom", grupo: "forma", looktype: 69479, looktypeShiny: 79479 },
    { pokeId: 11479, dex: 479, nome: "Frost Rotom", grupo: "forma", looktype: 71479, looktypeShiny: 81479 },
    { pokeId: 12479, dex: 479, nome: "Fan Rotom", grupo: "forma", looktype: 72479, looktypeShiny: 82479 },
    { pokeId: 8483, dex: 483, nome: "Dialga Origin", grupo: "forma", looktype: 68483, looktypeShiny: 78483 },
    { pokeId: 8484, dex: 484, nome: "Palkia Origin", grupo: "forma", looktype: 68484, looktypeShiny: 78484 },
    { pokeId: 8487, dex: 487, nome: "Giratina Origin", grupo: "forma", looktype: 68487, looktypeShiny: 78487 },
    { pokeId: 8492, dex: 492, nome: "Shaymin Sky", grupo: "forma", looktype: 68492, looktypeShiny: 78492 },
    { pokeId: 6503, dex: 503, nome: "Hisuian Samurott", grupo: "hisui", looktype: 66503, looktypeShiny: 76503 },
    { pokeId: 6549, dex: 549, nome: "Hisuian Lilligant", grupo: "hisui", looktype: 66549, looktypeShiny: 76549 },
    { pokeId: 5554, dex: 554, nome: "Galarian Darumaka", grupo: "galar", looktype: 65554, looktypeShiny: 75554 },
    { pokeId: 5555, dex: 555, nome: "Galarian Darmanitan", grupo: "galar", looktype: 65555, looktypeShiny: 75555 },
    { pokeId: 8555, dex: 555, nome: "Darmanitan Zen", grupo: "forma", looktype: 68555, looktypeShiny: 78555 },
    { pokeId: 9555, dex: 555, nome: "Galarian Darmanitan Zen", grupo: "forma", looktype: 69555, looktypeShiny: 79555 },
    { pokeId: 5562, dex: 562, nome: "Galarian Yamask", grupo: "galar", looktype: 65562, looktypeShiny: 75562 },
    { pokeId: 6570, dex: 570, nome: "Hisuian Zorua", grupo: "hisui", looktype: 66570, looktypeShiny: 76570 },
    { pokeId: 6571, dex: 571, nome: "Hisuian Zoroark", grupo: "hisui", looktype: 66571, looktypeShiny: 76571 },
    { pokeId: 5618, dex: 618, nome: "Galarian Stunfisk", grupo: "galar", looktype: 65618, looktypeShiny: 75618 },
    { pokeId: 6628, dex: 628, nome: "Hisuian Braviary", grupo: "hisui", looktype: 66628, looktypeShiny: 76628 },
    { pokeId: 8641, dex: 641, nome: "Tornadus Therian", grupo: "forma", looktype: 68641, looktypeShiny: 78641 },
    { pokeId: 8642, dex: 642, nome: "Thundurus Therian", grupo: "forma", looktype: 68642, looktypeShiny: 78642 },
    { pokeId: 8645, dex: 645, nome: "Landorus Therian", grupo: "forma", looktype: 68645, looktypeShiny: 78645 },
    { pokeId: 8646, dex: 646, nome: "White Kyurem", grupo: "forma", looktype: 68646, looktypeShiny: 78646 },
    { pokeId: 9646, dex: 646, nome: "Black Kyurem", grupo: "forma", looktype: 69646, looktypeShiny: 79646 },
    { pokeId: 8647, dex: 647, nome: "Keldeo Resolute", grupo: "forma", looktype: 68647, looktypeShiny: 78647 },
    { pokeId: 8648, dex: 648, nome: "Meloetta Pirouette", grupo: "forma", looktype: 68648, looktypeShiny: 78648 },
    { pokeId: 8681, dex: 681, nome: "Aegislash Blade", grupo: "forma", looktype: 68681, looktypeShiny: 78681 },
    { pokeId: 6705, dex: 705, nome: "Hisuian Sliggoo", grupo: "hisui", looktype: 66705, looktypeShiny: 76705 },
    { pokeId: 6706, dex: 706, nome: "Hisuian Goodra", grupo: "hisui", looktype: 66706, looktypeShiny: 76706 },
    { pokeId: 6713, dex: 713, nome: "Hisuian Avalugg", grupo: "hisui", looktype: 66713, looktypeShiny: 76713 },
    { pokeId: 8718, dex: 718, nome: "Zygarde 10%", grupo: "forma", looktype: 68718, looktypeShiny: 78718 },
    { pokeId: 9718, dex: 718, nome: "Zygarde Complete", grupo: "forma", looktype: 69718, looktypeShiny: 79718 },
    { pokeId: 8720, dex: 720, nome: "Hoopa Unbound", grupo: "forma", looktype: 68720, looktypeShiny: 78720 },
    { pokeId: 6724, dex: 724, nome: "Hisuian Decidueye", grupo: "hisui", looktype: 66724, looktypeShiny: 76724 },
    { pokeId: 8741, dex: 741, nome: "Oricorio Pom-Pom", grupo: "forma", looktype: 68741, looktypeShiny: 78741 },
    { pokeId: 9741, dex: 741, nome: "Oricorio Pa'u", grupo: "forma", looktype: 69741, looktypeShiny: 79741 },
    { pokeId: 11741, dex: 741, nome: "Oricorio Sensu", grupo: "forma", looktype: 71741, looktypeShiny: 81741 },
    { pokeId: 8745, dex: 745, nome: "Lycanroc Midnight", grupo: "forma", looktype: 68745, looktypeShiny: 78745 },
    { pokeId: 9745, dex: 745, nome: "Lycanroc Dusk", grupo: "forma", looktype: 69745, looktypeShiny: 79745 },
    { pokeId: 8746, dex: 746, nome: "Wishiwashi School", grupo: "forma", looktype: 68746, looktypeShiny: 78746 },
    { pokeId: 8774, dex: 774, nome: "Minior Core", grupo: "forma", looktype: 68774, looktypeShiny: 78774 },
    { pokeId: 8888, dex: 888, nome: "Zacian Crowned", grupo: "forma", looktype: 68888, looktypeShiny: 78888 },
    { pokeId: 8889, dex: 889, nome: "Zamazenta Crowned", grupo: "forma", looktype: 68889, looktypeShiny: 78889 },
    { pokeId: 8898, dex: 898, nome: "Calyrex Ice Rider", grupo: "forma", looktype: 68898, looktypeShiny: 78898 },
    { pokeId: 9898, dex: 898, nome: "Calyrex Shadow Rider", grupo: "forma", looktype: 69898, looktypeShiny: 79898 },
    { pokeId: 8901, dex: 901, nome: "Bloodmoon Ursaluna", grupo: "forma", looktype: 68901, looktypeShiny: 78901 },
    { pokeId: 8902, dex: 902, nome: "Basculegion Female", grupo: "forma", looktype: 68902, looktypeShiny: 78902 },
    { pokeId: 8905, dex: 905, nome: "Enamorus Therian", grupo: "forma", looktype: 68905, looktypeShiny: 78905 },
    { pokeId: 8964, dex: 964, nome: "Palafin Hero", grupo: "forma", looktype: 68964, looktypeShiny: 78964 },
    { pokeId: 9017, dex: 1017, nome: "Ogerpon Wellspring", grupo: "forma", looktype: 69017, looktypeShiny: 79017 },
    { pokeId: 8017, dex: 1017, nome: "Ogerpon Hearthflame", grupo: "forma", looktype: 68017, looktypeShiny: 78017 },
    { pokeId: 12017, dex: 1017, nome: "Ogerpon Cornerstone", grupo: "forma", looktype: 72017, looktypeShiny: 82017 }
  ];
  var FORMA_POR_POKEID = new Map(FORMAS.map((f) => [f.pokeId, f]));
  var dexBaseDaForma = (pokeId) => FORMA_POR_POKEID.get(Number(pokeId))?.dex ?? null;

  // tests/.cache/shared/mistico.mjs
  var CAPTURA_MISTICA_POR_BOLA = Object.freeze({
    1: 1e-3,
    // Poke Ball
    2: 2e-3,
    // Great Ball
    3: 3e-3,
    // Super Ball
    4: 4e-3,
    // Ultra Ball
    5: 5e-3
    // Beast Ball
  });
  var BOLAS_MISTICAS_DA_MELHOR = Object.freeze([5, 4, 3, 2, 1]);
  var LENDARIOS_KANTO_JOHTO = Object.freeze([144, 145, 146, 150, 243, 244, 245, 249, 250]);
  var MITICOS_KANTO_JOHTO = Object.freeze([151, 251]);
  var MITICOS_FORA_DO_FILTRO = Object.freeze([494]);
  var DEX_LENDARIO_OU_MITICO = /* @__PURE__ */ new Set([
    ...LENDARIOS_KANTO_JOHTO,
    ...MITICOS_KANTO_JOHTO,
    ...MITICOS_FORA_DO_FILTRO,
    ...LENDARIO_DEX,
    ...MITICO_DEX
  ]);
  var DEX_MITICO = /* @__PURE__ */ new Set([
    ...MITICOS_KANTO_JOHTO,
    ...MITICOS_FORA_DO_FILTRO,
    ...MITICO_DEX
  ]);
  var dexNacional = (id) => {
    if (id >= 13e3) return id % 1e3;
    if (isMegaPokeId(id)) return megaPorPokeId(id)?.dex ?? id - MEGA_POKE_BASE;
    return dexBaseDaForma(id) ?? id;
  };
  function ehLendarioOuMitico(pokeId) {
    const id = Number(pokeId);
    return Number.isInteger(id) && id > 0 && DEX_LENDARIO_OU_MITICO.has(dexNacional(id));
  }

  // tests/.cache/shared/nota-pokemon.mjs
  var IV_POR_STAT = { min: 1, max: 32 };
  var IV_STATS = 6;
  var IV_MIN = IV_STATS * IV_POR_STAT.min;
  var IV_MAX = IV_STATS * IV_POR_STAT.max;
  var BANDAS_QUALIDADE = [
    { min: 0.8, max: 0.9, chance: 5 },
    { min: 0.9, max: 1, chance: 5 },
    { min: 1, max: 1.1, chance: 34.03846 },
    { min: 1.1, max: 1.2, chance: 20 },
    { min: 1.2, max: 1.3, chance: 10 },
    { min: 1.3, max: 1.4, chance: 10 },
    { min: 1.4, max: 1.5, chance: 10 },
    { min: 1.5, max: 1.6, chance: 5 },
    { min: 1.7, max: 1.8, chance: 0.67308 },
    { min: 1.8, max: 1.8, chance: 0.28846 }
  ];
  var QUALIDADE_MIN = BANDAS_QUALIDADE[0].min;
  var QUALIDADE_MAX = BANDAS_QUALIDADE.at(-1).max;
  var EXPO_QUALIDADE = {
    hp: 0.95,
    atk: 0.8,
    def: 0.8,
    spAtk: 0.8,
    spDef: 0.8,
    speed: 0.95
  };
  var BONUS_POTENCIA = [0, 0.05, 0.1, 0.25, 1];
  var multPotencia = (n) => 1 + (BONUS_POTENCIA[(Math.round(n) || 1) - 1] ?? 0);
  var MULT_SHINY_STATS = 3;
  var MULT_LENDARIO_STATS = 4;
  var multDeNascenca = (potencia, shiny, lendario = false) => multPotencia(potencia) * (shiny ? MULT_SHINY_STATS : 1) * (lendario ? MULT_LENDARIO_STATS : 1);

  // tests/.cache/shared/tm-elemental.mjs
  var TM_ELEMENTAL_POWER = 2800;
  var TM_ELEMENTAL_POWER_PVP = 1800;
  var TM_ELEMENTAL_CD_MS = 6e4;
  var golpeTmElemental = (tipo, fisico = false, { pvp = false } = {}) => ({
    name: `Z-Move ${tipo}`,
    power: pvp === true ? TM_ELEMENTAL_POWER_PVP : TM_ELEMENTAL_POWER,
    type: tipo,
    category: fisico === true ? "PHYSICAL" : "SPECIAL",
    cooldownMs: TM_ELEMENTAL_CD_MS,
    tmElemental: true
  });
  var ZMOVE_TROCA_COOLDOWN_MS = 5 * 6e4;

  // tests/.cache/shared/pvp-tabuleiro.mjs
  var TABULEIRO_MAX = 5;
  var ARENA_TABULEIRO = Object.freeze({
    slug: "ginasio-duelo",
    mapa: "cerulean",
    box: Object.freeze([14, -27, 29, 24]),
    groundZ: 7
  });
  var FILEIRAS = Object.freeze([
    Object.freeze({ cy: 13, de: 12, ate: 17 }),
    Object.freeze({ cy: 14, de: 11, ate: 18 }),
    Object.freeze({ cy: 15, de: 11, ate: 18 }),
    Object.freeze({ cy: 16, de: 13, ate: 16 }),
    Object.freeze({ cy: 17, de: 13, ate: 16 })
  ]);
  var FILEIRAS_TABULEIRO = FILEIRAS.length;
  var TREINADOR_BAIXO = Object.freeze({ cx: 15, cy: 18 });
  var CASAS_TABULEIRO = Object.freeze((() => {
    const lista = [];
    FILEIRAS.forEach((f, fileira) => {
      for (let cx = f.de; cx <= f.ate; cx++) {
        lista.push(Object.freeze({ casa: lista.length, fileira, coluna: cx - f.de, cx, cy: f.cy }));
      }
    });
    return lista;
  })());
  var TOTAL_CASAS = CASAS_TABULEIRO.length;
  var ALCANCES_A_DISTANCIA = Object.freeze([2, 3, 4]);
  var F0 = [2, 3, 1, 4, 0, 5];
  var F1 = [9, 10, 8, 11, 7, 12, 6, 13];
  var F2_MIOLO = [17, 18, 16, 19];
  var F2_LADOS = [15, 20];
  var F2_PONTAS = [14, 21];
  var F2 = [...F2_MIOLO, ...F2_LADOS, ...F2_PONTAS];
  var F3 = [23, 24, 22, 25];
  var F4 = [27, 28, 26, 29];
  var PRIORIDADE_POR_ALCANCE = Object.freeze({
    1: Object.freeze([...F0, ...F1, ...F2, ...F3, ...F4]),
    2: Object.freeze([...F1, ...F2, ...F0, ...F3, ...F4]),
    3: Object.freeze([...F2_MIOLO, ...F3, ...F2_LADOS, ...F4, ...F2_PONTAS, ...F1, ...F0]),
    4: Object.freeze([...F3, ...F2, ...F4, ...F1, ...F0])
  });
  function casaValida(v) {
    if (typeof v === "string" && v.trim() !== "") v = Number(v);
    return Number.isInteger(v) && v >= 0 && v < TOTAL_CASAS ? v : null;
  }
  function normalizarMapaDeCasas(bruto, ids) {
    const saida = {};
    if (!bruto || typeof bruto !== "object" || Array.isArray(bruto)) return saida;
    const usadas = /* @__PURE__ */ new Set();
    for (const id of (ids ?? []).slice(0, TABULEIRO_MAX)) {
      const chave = String(id);
      if (!Object.prototype.hasOwnProperty.call(bruto, chave)) continue;
      const c = casaValida(bruto[chave]);
      if (c == null || usadas.has(c)) continue;
      usadas.add(c);
      saida[chave] = c;
    }
    return saida;
  }

  // tests/.cache/shared/ginasios.mjs
  var TIPOS_GINASIO = [
    "NORMAL",
    "FIRE",
    "WATER",
    "GRASS",
    "ELECTRIC",
    "ICE",
    "FIGHTING",
    "POISON",
    "GROUND",
    "FLYING",
    "PSYCHIC",
    "BUG",
    "ROCK",
    "GHOST",
    "DRAGON",
    "DARK",
    "STEEL",
    "FAIRY"
  ];
  var TIPOS_VALIDOS = new Set(TIPOS_GINASIO);
  var iconeDoGinasio = (tipo) => `/img/tipos/${String(tipo).toLowerCase()}.svg`;
  var GINASIO_TIME_MAX = 5;
  var ARENA_NIVEL_CHEIO = 150;
  var ARENA_EXPO_GINASIO = 0.45;
  var ARENA_EXPO_GUERRA = 0.2;
  var GINASIO_NIVEL_MIN = 150;
  var GINASIO_BUFF_PODIO_PCT = Object.freeze([25, 10, 5]);
  var GINASIO_BUFF_PCT = GINASIO_BUFF_PODIO_PCT[0];
  var GINASIO_BUFF_MULT = 1 + GINASIO_BUFF_PCT / 100;
  function segundoETerceiro(times, liderId) {
    const resto = (times ?? []).filter((t) => Number(t.playerId) !== Number(liderId));
    return { segundoId: resto[0]?.playerId ?? null, terceiroId: resto[1]?.playerId ?? null };
  }
  var pctDoLugarGinasio = (lugar) => GINASIO_BUFF_PODIO_PCT[Math.floor(Number(lugar)) - 1] ?? 0;
  var multDoLugarGinasio = (lugar) => 1 + pctDoLugarGinasio(lugar) / 100;
  var GINASIO_DESAFIO_COOLDOWN_MS = 24 * 60 * 60 * 1e3;
  var SLUG_ARENA_GINASIO = "ginasio-duelo";
  function tipoDeGinasio(bruto) {
    const t = String(bruto ?? "").toUpperCase();
    return TIPOS_VALIDOS.has(t) ? t : null;
  }
  var pokemonDoTipo = (tipos, tipo) => (tipos ?? []).some((x) => String(x).toUpperCase() === String(tipo).toUpperCase());
  function nivelDeArena(level, expoente) {
    const n = Math.max(1, Math.floor(Number(level) || 1));
    if (n <= ARENA_NIVEL_CHEIO) return n;
    return Math.max(
      ARENA_NIVEL_CHEIO,
      Math.round(ARENA_NIVEL_CHEIO * (n / ARENA_NIVEL_CHEIO) ** expoente)
    );
  }
  var nivelNoGinasio = (level) => nivelDeArena(level, ARENA_EXPO_GINASIO);
  var nivelNaGuerra = (level) => nivelDeArena(level, ARENA_EXPO_GUERRA);
  var hpDeCombateGinasio = (hp) => Math.max(24, Math.round(hp * 12));
  function statsNoNivel(pk, esp, nivelBruto) {
    if (!esp || !pk?.ivs) return null;
    const nivel = Math.max(1, Math.floor(Number(nivelBruto) || 1));
    const mult = multDeNascenca(pk.potencia, pk.shiny, ehLendarioOuMitico(esp?.pokeId));
    const b = basesComRefino(esp, pk.refino);
    const um = (base, iv, k) => Math.round((base + 2 * (iv ?? 1)) * (nivel / 100) * (Math.pow(Number(pk.quality ?? pk.qualidade) || 1, EXPO_QUALIDADE[k]) * mult));
    return {
      hp: um(b.hp, pk.ivs.hp, "hp"),
      atk: um(b.atk, pk.ivs.atk, "atk"),
      def: um(b.def, pk.ivs.def, "def"),
      spAtk: um(b.spAtk, pk.ivs.spAtk, "spAtk"),
      spDef: um(b.spDef, pk.ivs.spDef, "spDef"),
      speed: um(b.speed, pk.ivs.speed, "speed")
    };
  }
  function statsNoGinasio(pk, esp) {
    return statsNoNivel(pk, esp, nivelNoGinasio(pk?.level));
  }
  var GOLPE_BASICO = { power: 30, cooldownMs: 2e3, category: "PHYSICAL" };
  function notaGolpeGinasio(g, nivel, stats, tipos) {
    const atk = g.category === "SPECIAL" ? stats.spAtk : stats.atk;
    const def = g.category === "SPECIAL" ? stats.spDef : stats.def;
    const stab = tipos.includes(g.type) ? 1.5 : 1;
    const base = (2 * nivel / 5 + 2) * (g.power ?? 0) * (atk / Math.max(1, def)) / 50 + 2;
    return base * stab;
  }
  function melhorGolpeDoGinasio(esp, nivel, stats = null, tmElemental = null, tmFisico = false, { pvp = false } = {}) {
    const golpes = (esp?.attacks ?? []).filter((a) => (a.learnLevel ?? 1) <= nivel);
    if (tmElemental) golpes.push(golpeTmElemental(tmElemental, tmFisico, { pvp }));
    if (!golpes.length) return GOLPE_BASICO;
    if (stats) {
      const tipos = [esp.type1, esp.type2].filter(Boolean);
      let melhor2 = GOLPE_BASICO;
      let melhorNota = -1;
      for (const g of golpes) {
        const nota2 = notaGolpeGinasio(g, nivel, stats, tipos);
        if (nota2 > melhorNota) {
          melhorNota = nota2;
          melhor2 = g;
        }
      }
      return melhor2;
    }
    let melhor = GOLPE_BASICO;
    let nota = GOLPE_BASICO.power / GOLPE_BASICO.cooldownMs;
    for (const a of golpes) {
      const n = (a.power ?? 0) / Math.max(1, a.cooldownMs ?? 1e4);
      if (n > nota) {
        nota = n;
        melhor = a;
      }
    }
    return melhor;
  }
  function ratingDeCombate(pk, esp) {
    return ratingNoNivel(pk, esp, nivelNoGinasio(pk?.level), true);
  }
  var poderNoNivel = (pk, esp, nivel, { pvp = false } = {}) => esp ? Math.max(1, Math.round(ratingNoNivel(pk, esp, nivel, pvp) / GINASIO_ESCALA_FORCA)) : 1;
  function ratingNoNivel(pk, esp, nivelBruto, pvp = false) {
    const st = statsNoNivel(pk, esp, nivelBruto);
    if (!st) return 0;
    const nivel = Math.max(1, Math.floor(Number(nivelBruto) || 1));
    const g = melhorGolpeDoGinasio(esp, nivel, st, pk.tmElemental, pk.tmFisico, { pvp });
    const atk = Math.max(st.atk, st.spAtk);
    const def = (st.def + st.spDef) / 2;
    const dps = (2 * nivel / 5 + 2) * (g.power ?? GOLPE_BASICO.power) * atk / 50 / (Math.max(400, g.cooldownMs ?? GOLPE_BASICO.cooldownMs) / 1e3);
    return dps * hpDeCombateGinasio(st.hp) * def;
  }
  var VARIACAO_DANO_MIN = 0.85;
  var VARIACAO_DANO_MAX = 1;
  function metricasDeCombate(pk, esp) {
    const st = statsNoGinasio(pk, esp);
    if (!st) return null;
    const nivel = nivelNoGinasio(pk.level);
    const g = melhorGolpeDoGinasio(esp, nivel, st, pk.tmElemental, pk.tmFisico, { pvp: true });
    const atk = Math.max(st.atk, st.spAtk);
    const def = (st.def + st.spDef) / 2;
    const dps = (2 * nivel / 5 + 2) * (g.power ?? GOLPE_BASICO.power) * atk / 50 / (Math.max(400, g.cooldownMs ?? GOLPE_BASICO.cooldownMs) / 1e3);
    const ehp = hpDeCombateGinasio(st.hp);
    const rating = dps * ehp * def;
    return {
      dps,
      ehp,
      def,
      rating,
      golpe: g,
      stats: st,
      nivel,
      nivelReal: Math.max(1, Math.floor(Number(pk.level) || 1)),
      tm: pk.tmElemental ?? null,
      golpeDaTm: !!g.tmElemental
    };
  }
  var tiposDeEspecie = (esp) => [esp?.type1, esp?.type2].filter(Boolean);
  function basicoContra(tipos, tiposDefensor, tabela) {
    for (const tipo of tipos.length ? tipos : ["NORMAL"]) {
      if (efetividade(tipo, tiposDefensor[0], tiposDefensor[1], tabela) > 0) {
        return { ...GOLPE_BASICO, type: tipo };
      }
    }
    return null;
  }
  function notaGolpeComMatchup(g, nivel, stats, tiposAtacante, tiposDefensor, defStats, tabela) {
    const ef = efetividade(g.type, tiposDefensor[0], tiposDefensor[1], tabela);
    if (ef === 0) return -1;
    const stab = tiposAtacante.includes(g.type) ? 1.5 : 1;
    const atk = g.category === "SPECIAL" ? stats.spAtk : stats.atk;
    const def = defStats ? g.category === "SPECIAL" ? defStats.spDef : defStats.def : atk;
    const base = (2 * nivel / 5 + 2) * (g.power ?? 0) * (atk / Math.max(1, def)) / 50 + 2;
    return base * stab * ef;
  }
  function melhorGolpeContra(esp, nivel, stats, tiposDefensor, defStats, tabela, tmElemental = null, tmFisico = false) {
    const tipos = tiposDeEspecie(esp);
    const golpes = (esp?.attacks ?? []).filter((a) => (a.learnLevel ?? 1) <= nivel);
    if (tmElemental) golpes.push(golpeTmElemental(tmElemental, tmFisico, { pvp: true }));
    let melhor = null;
    let melhorNota = -1;
    for (const g of golpes) {
      const nota = notaGolpeComMatchup(g, nivel, stats, tipos, tiposDefensor, defStats, tabela);
      if (nota > melhorNota) {
        melhorNota = nota;
        melhor = g;
      }
    }
    if (melhor) return melhor;
    return basicoContra(tipos, tiposDefensor, tabela) ?? GOLPE_BASICO;
  }
  function metricasDeCombateContra(pk, esp, pkOpp, espOpp, tabela) {
    const st = statsNoGinasio(pk, esp);
    const stOpp = statsNoGinasio(pkOpp, espOpp);
    if (!st || !stOpp) return null;
    const nivel = nivelNoGinasio(pk.level);
    const tiposDef = tiposDeEspecie(espOpp);
    const tiposAtq = tiposDeEspecie(esp);
    const g = melhorGolpeContra(esp, nivel, st, tiposDef, stOpp, tabela, pk.tmElemental, pk.tmFisico);
    const atk = g.category === "SPECIAL" ? st.spAtk : st.atk;
    const defOpp = g.category === "SPECIAL" ? stOpp.spDef : stOpp.def;
    const ef = efetividade(g.type, tiposDef[0], tiposDef[1], tabela);
    const stab = tiposAtq.includes(g.type) ? 1.5 : 1;
    const base = (2 * nivel / 5 + 2) * (g.power ?? GOLPE_BASICO.power) * atk / Math.max(1, defOpp) / 50 + 2;
    const dano = base * stab * Math.max(0, ef);
    const cd = Math.max(400, g.cooldownMs ?? GOLPE_BASICO.cooldownMs) / 1e3;
    const dps = dano / cd;
    const ehp = hpDeCombateGinasio(st.hp);
    const defTank = (st.def + st.spDef) / 2;
    const rating = dps * ehp * defTank;
    return {
      dps,
      ehp,
      def: defTank,
      rating,
      golpe: g,
      stats: st,
      nivel,
      nivelReal: Math.max(1, Math.floor(Number(pk.level) || 1)),
      ef,
      stab: stab > 1,
      // `tm` é o TIPO do disco que o bicho tem; `golpeDaTm` diz se foi ELE que a conta usou.
      // São coisas diferentes na tela: ter a TM e não usá-la contra aquele oponente é
      // informação, não detalhe.
      tm: pk.tmElemental ?? null,
      golpeDaTm: !!g.tmElemental
    };
  }
  function rngDeterministico(seed) {
    let s = seed >>> 0;
    return () => {
      s = Math.imul(s, 1664525) + 1013904223 >>> 0;
      return s / 4294967296;
    };
  }
  function compararDuelo1v1(pkA, espA, pkB, espB, { amostras = 400, tabelaTipos = null } = {}) {
    const usaTipos = tabelaTipos && Object.keys(tabelaTipos).length > 0;
    const mA = usaTipos ? metricasDeCombateContra(pkA, espA, pkB, espB, tabelaTipos) : metricasDeCombate(pkA, espA);
    const mB = usaTipos ? metricasDeCombateContra(pkB, espB, pkA, espA, tabelaTipos) : metricasDeCombate(pkB, espB);
    if (!mA || !mB) return null;
    const scoreA = (mA2, v) => usaTipos ? mA2.dps * v * mA2.ehp : mA2.dps * v * mA2.ehp * mA2.def;
    const scoreB = (mB2, v) => usaTipos ? mB2.dps * v * mB2.ehp : mB2.dps * v * mB2.ehp * mB2.def;
    const rAmin = scoreA(mA, VARIACAO_DANO_MIN);
    const rAmax = scoreA(mA, VARIACAO_DANO_MAX);
    const rBmin = scoreB(mB, VARIACAO_DANO_MIN);
    const rBmax = scoreB(mB, VARIACAO_DANO_MAX);
    const baseA = scoreA(mA, 1);
    const baseB = scoreB(mB, 1);
    const espelho = Math.abs(baseA - baseB) / Math.max(baseA, baseB, 1) < 1e-6;
    let pctA;
    let pctB;
    if (espelho) {
      pctA = 0.5;
      pctB = 0.5;
    } else {
      const seed = (pkA.speciesId ?? 0) * 997 + (pkB.speciesId ?? 0) * 991 + Math.round(mA.rating) + Math.round(mB.rating * 3) >>> 0;
      const rnd = rngDeterministico(seed);
      let vitA = 0;
      for (let i = 0; i < amostras; i++) {
        const vA = VARIACAO_DANO_MIN + rnd() * (VARIACAO_DANO_MAX - VARIACAO_DANO_MIN);
        const vB = VARIACAO_DANO_MIN + rnd() * (VARIACAO_DANO_MAX - VARIACAO_DANO_MIN);
        const rA = scoreA(mA, vA);
        const rB = scoreB(mB, vB);
        if (rA > rB) vitA++;
        else if (rA === rB) vitA += 0.5;
      }
      pctA = vitA / amostras;
      pctB = 1 - pctA;
    }
    let pctMinA = pctA;
    let pctMaxA = pctA;
    if (rAmin > rBmax) {
      pctMinA = 1;
      pctMaxA = 1;
    } else if (rAmax < rBmin) {
      pctMinA = 0;
      pctMaxA = 0;
    } else {
      pctMinA = Math.min(pctA, rAmin / (rAmin + rBmax));
      pctMaxA = Math.max(pctA, rAmax / (rAmax + rBmin));
    }
    const favorito = mA.rating >= mB.rating ? "a" : "b";
    return {
      a: mA,
      b: mB,
      poderA: Math.max(1, Math.round(mA.rating / GINASIO_ESCALA_FORCA)),
      poderB: Math.max(1, Math.round(mB.rating / GINASIO_ESCALA_FORCA)),
      pctA,
      pctB,
      pctMinA: Math.max(0, Math.min(1, pctMinA)),
      pctMaxA: Math.max(0, Math.min(1, pctMaxA)),
      favorito,
      empate: espelho || Math.abs(mA.rating - mB.rating) / Math.max(mA.rating, mB.rating, 1) < 0.015
    };
  }
  var GINASIO_ESCALA_FORCA = 1e7;
  var poderNoGinasio = (pk, esp = null) => esp ? Math.max(1, Math.round(ratingDeCombate(pk, esp) / GINASIO_ESCALA_FORCA)) : 1;
  var poderDoTimeGinasio = (pokemons, esps = null) => (pokemons ?? []).reduce(
    (s, pk) => s + poderNoGinasio(pk, esps?.get?.(pk?.speciesId) ?? null),
    0
  );
  function motivoForaDoGinasio(pk, tipo, escolhidos = [], emGinasio = null) {
    if (!pk) return "sumiu";
    if (!pokemonDoTipo(pk.tipos, tipo)) return "tipo";
    if (outroGinasioDoPokemon(pk, tipo, emGinasio)) return "outroGinasio";
    if (escolhidos.some((o) => o && o.id !== pk.id && o.speciesId === pk.speciesId)) return "especie";
    if (escolhidos.some((o) => o && o.id === pk.id)) return "repetido";
    if (escolhidos.length >= GINASIO_TIME_MAX) return "cheio";
    return null;
  }
  function timeValidoDoGinasio(pokemons, tipo) {
    const ok = [];
    for (const pk of pokemons ?? []) {
      if (!motivoForaDoGinasio(pk, tipo, ok)) ok.push(pk);
    }
    return ok;
  }
  function validarTimeGinasio(pokemons, tipo, emGinasio = null) {
    if (!tipoDeGinasio(tipo)) return { ok: false, motivo: "tipo" };
    const lista = pokemons ?? [];
    if (lista.length > GINASIO_TIME_MAX) return { ok: false, motivo: "cheio" };
    const ok = [];
    for (const pk of lista) {
      const motivo = motivoForaDoGinasio(pk, tipo, ok, emGinasio);
      if (motivo) {
        return {
          ok: false,
          motivo,
          pk: pk ?? null,
          ...motivo === "outroGinasio" ? { outroTipo: outroGinasioDoPokemon(pk, tipo, emGinasio) } : {}
        };
      }
      ok.push(pk);
    }
    return { ok: true, motivo: null };
  }
  var ORDEM_DOS_GINASIOS = new Map(TIPOS_GINASIO.map((t, i) => [t, i]));
  function ginasioDeCadaPokemon(linhas) {
    const melhor = /* @__PURE__ */ new Map();
    for (const l of linhas ?? []) {
      const tipo = tipoDeGinasio(l?.tipo);
      if (!tipo) continue;
      const dono = Number(l.playerId);
      const em = new Date(l.atualizadoEm ?? 0).getTime() || 0;
      const ordem = ORDEM_DOS_GINASIOS.get(tipo);
      let doDono = melhor.get(dono);
      if (!doDono) melhor.set(dono, doDono = /* @__PURE__ */ new Map());
      for (const bruto of l.ids ?? []) {
        const id = Number(bruto);
        if (!Number.isFinite(id)) continue;
        const atual = doDono.get(id);
        if (!atual || em > atual.em || em === atual.em && ordem < atual.ordem) doDono.set(id, { tipo, em, ordem });
      }
    }
    const saida = /* @__PURE__ */ new Map();
    for (const [dono, doDono] of melhor) saida.set(dono, new Map([...doDono].map(([id, v]) => [id, v.tipo])));
    return saida;
  }
  function idsQueContamNoGinasio(linha, ginasioDe) {
    const tipo = tipoDeGinasio(linha?.tipo);
    const doDono = ginasioDe?.get(Number(linha?.playerId));
    if (!tipo || !doDono) return [];
    const saida = [];
    for (const bruto of linha.ids ?? []) {
      const id = Number(bruto);
      if (Number.isFinite(id) && doDono.get(id) === tipo && !saida.includes(id)) saida.push(id);
    }
    return saida;
  }
  function timesQueContam(linhas, porId, ginasioDe = ginasioDeCadaPokemon(linhas)) {
    return (linhas ?? []).map((l) => {
      const dono = Number(l.playerId);
      const candidatos = [];
      for (const id of idsQueContamNoGinasio(l, ginasioDe)) {
        const pk = porId?.get(id);
        if (pk && Number(pk.donoId) === dono) candidatos.push(pk);
      }
      const validos = timeValidoDoGinasio(candidatos, l.tipo);
      const casas = normalizarMapaDeCasas(l.casas, validos.map((pk) => pk.id));
      const pokemons = validos.map((pk) => ({ ...pk, casa: casas[String(pk.id)] ?? null }));
      return { pokemons, casas };
    });
  }
  function pokemonEmGinasioDosTimes(times) {
    const saida = {};
    for (const time of times ?? []) {
      const tipo = tipoDeGinasio(time?.tipo);
      if (!tipo) continue;
      for (const pk of time.pokemons ?? []) if (pk?.id != null) saida[String(pk.id)] = tipo;
    }
    return saida;
  }
  function outroGinasioDoPokemon(pk, tipo, emGinasio) {
    if (!emGinasio || pk == null) return null;
    const id = typeof pk === "object" ? pk.id : pk;
    if (id == null) return null;
    const onde = emGinasio instanceof Map ? emGinasio.get(id) ?? emGinasio.get(Number(id)) ?? emGinasio.get(String(id)) : emGinasio[String(id)];
    const outro = tipoDeGinasio(onde);
    return outro && outro !== tipoDeGinasio(tipo) ? outro : null;
  }
  function lugarNoTipo(podio, tipo) {
    if (podio instanceof Map) return podio.get(tipo);
    if (podio instanceof Set) return podio.has(tipo) ? 1 : void 0;
    if (Array.isArray(podio)) return podio.includes(tipo) ? 1 : void 0;
    return podio[tipo];
  }
  function buffGinasioDoPokemon(tiposPokemon, podio) {
    if (!podio || !tiposPokemon?.length) return null;
    let melhor = 0;
    for (const tipo of tiposPokemon) {
      const lugar = Math.floor(Number(lugarNoTipo(podio, String(tipo).toUpperCase())) || 0);
      if (pctDoLugarGinasio(lugar) > 0 && (!melhor || lugar < melhor)) melhor = lugar;
    }
    return melhor ? { lugar: melhor, pct: pctDoLugarGinasio(melhor) } : null;
  }
  function multBuffGinasio(tiposPokemon, podio) {
    const b = buffGinasioDoPokemon(tiposPokemon, podio);
    return b ? multDoLugarGinasio(b.lugar) : 1;
  }

  // tests/.cache/shared/dia-sazonal.mjs
  var dia_sazonal_exports = {};
  __export(dia_sazonal_exports, {
    ANTECIPACAO_SAZONAL_MS: () => ANTECIPACAO_SAZONAL_MS,
    MULT_SAZONAL: () => MULT_SAZONAL,
    TIPOS_SAZONAIS: () => TIPOS_SAZONAIS,
    chaveDoDiaSazonal: () => chaveDoDiaSazonal,
    danoSazonal: () => danoSazonal,
    ehDoTipoSazonal: () => ehDoTipoSazonal,
    fimDoDiaSazonal: () => fimDoDiaSazonal,
    inicioDoDiaSazonal: () => inicioDoDiaSazonal,
    multDanoSazonal: () => multDanoSazonal
  });

  // tests/.cache/shared/fuso-brasilia.mjs
  var FUSO_BRASILIA_MS = 3 * 60 * 6e4;

  // tests/.cache/shared/dia-sazonal.mjs
  var TIPOS_SAZONAIS = Object.freeze([
    "NORMAL",
    "FIRE",
    "WATER",
    "GRASS",
    "ELECTRIC",
    "ICE",
    "FIGHTING",
    "POISON",
    "GROUND",
    "FLYING",
    "PSYCHIC",
    "BUG",
    "ROCK",
    "GHOST",
    "DRAGON",
    "DARK",
    "STEEL",
    "FAIRY"
  ]);
  var MULT_SAZONAL = 2;
  var DIA_MS = 864e5;
  var ANTECIPACAO_SAZONAL_MS = 6 * 36e5;
  var DESLOCAMENTO_MS = FUSO_BRASILIA_MS - ANTECIPACAO_SAZONAL_MS;
  var chaveDoDiaSazonal = (t) => new Date(Number(t) - DESLOCAMENTO_MS).toISOString().slice(0, 10);
  var inicioDoDiaSazonal = (t) => Math.floor((Number(t) - DESLOCAMENTO_MS) / DIA_MS) * DIA_MS + DESLOCAMENTO_MS;
  var fimDoDiaSazonal = (t) => inicioDoDiaSazonal(t) + DIA_MS;
  var ehDoTipoSazonal = (tipos, tipo) => !!tipo && Array.isArray(tipos) && tipos.includes(tipo);
  function multDanoSazonal(tipo, tiposAtacante, tiposDefensor) {
    if (!tipo) return 1;
    const ataca = ehDoTipoSazonal(tiposAtacante, tipo) ? MULT_SAZONAL : 1;
    const apanha = ehDoTipoSazonal(tiposDefensor, tipo) ? MULT_SAZONAL * MULT_SAZONAL : 1;
    return ataca / apanha;
  }
  var danoSazonal = (dano, mult) => mult === 1 ? dano : Math.max(dano > 0 ? 1 : 0, Math.round(dano * mult));

  // tests/.cache/shared/captura-degrau.mjs
  var captura_degrau_exports = {};
  __export(captura_degrau_exports, {
    FATOR_CAPTURA_DO_DEGRAU: () => FATOR_CAPTURA_DO_DEGRAU,
    capturaPeloDegrau: () => capturaPeloDegrau,
    chaveDeHunt: () => chaveDeHunt,
    chavesEmHunt: () => chavesEmHunt,
    fatorDeCaptura: () => fatorDeCaptura
  });
  var FATOR_CAPTURA_DO_DEGRAU = Object.freeze({ 20: 2.8669, 40: 3.5745, 100: 4.4681 });
  var chaveDeHunt = (pokeId) => dexDe2(Number(pokeId));
  function chavesEmHunt(hunts) {
    const chaves = /* @__PURE__ */ new Set();
    for (const h of hunts ?? []) for (const e of h.especies ?? []) chaves.add(chaveDeHunt(e.pokeId));
    return chaves;
  }
  var dexBase = (id) => id >= 13e3 ? id % 1e3 : dexBaseDaForma(id) ?? id;
  function capturaPeloDegrau(pokeId, chaves) {
    const id = Number(pokeId);
    if (!Number.isFinite(id) || !chaves?.has(chaveDeHunt(id))) return false;
    return !DEX_LENDARIO_OU_MITICO.has(dexBase(id));
  }
  function fatorDeCaptura({ tetoCaptura, peloDegrau, preco, raridadeDiv = 17, porDegrau = FATOR_CAPTURA_DO_DEGRAU }) {
    if (peloDegrau) {
      const f = Number(porDegrau?.[tetoCaptura]) || Number(porDegrau?.[100]);
      return f > 0 ? f : FATOR_CAPTURA_DO_DEGRAU[100];
    }
    const raridade = Math.log10(Math.max(100, Number(preco) || 0)) - 1;
    return 1 + raridade ** 3 / raridadeDiv;
  }

  // tests/.cache/shared/ovo-shiny.mjs
  var ovo_shiny_exports = {};
  __export(ovo_shiny_exports, {
    CHANCE_FRAGMENTO_OVO_SHINY_BASE: () => CHANCE_FRAGMENTO_OVO_SHINY_BASE,
    CHANCE_FRAGMENTO_OVO_SHINY_PASSO: () => CHANCE_FRAGMENTO_OVO_SHINY_PASSO,
    CUSTO_FRAGMENTOS_OVO_SHINY: () => CUSTO_FRAGMENTOS_OVO_SHINY,
    DEGRAUS_FRAGMENTO_OVO_SHINY: () => DEGRAUS_FRAGMENTO_OVO_SHINY,
    FRAGMENTO_OVO_SHINY_ID: () => FRAGMENTO_OVO_SHINY_ID,
    HORAS_OVO_SHINY: () => HORAS_OVO_SHINY,
    NIVEL_MIN_FRAGMENTO_OVO_SHINY: () => NIVEL_MIN_FRAGMENTO_OVO_SHINY,
    OVO_SHINY_ITEM_ID: () => OVO_SHINY_ITEM_ID,
    chanceDoDegrauOvoShiny: () => chanceDoDegrauOvoShiny,
    chanceFragmentoOvoShiny: () => chanceFragmentoOvoShiny,
    degrauFragmentoOvoShiny: () => degrauFragmentoOvoShiny,
    regiaoDoNivelOvoShiny: () => regiaoDoNivelOvoShiny,
    tabelaFragmentoOvoShiny: () => tabelaFragmentoOvoShiny
  });
  var FRAGMENTO_OVO_SHINY_ID = 70017;
  var OVO_SHINY_ITEM_ID = 70094;
  var CUSTO_FRAGMENTOS_OVO_SHINY = 10;
  var HORAS_OVO_SHINY = 48;
  var CHANCE_FRAGMENTO_OVO_SHINY_BASE = 1 / 8e4;
  var CHANCE_FRAGMENTO_OVO_SHINY_PASSO = 5e-7;
  var NIVEL_MIN_FRAGMENTO_OVO_SHINY = 5e3;
  var DEGRAUS_FRAGMENTO_OVO_SHINY = Object.freeze([
    // Unova
    5e3,
    5400,
    5900,
    6400,
    6950,
    8e3,
    8600,
    // Kalos
    1e4,
    11250,
    12750,
    14350,
    16e3,
    2e4,
    21200,
    // Alola
    25e3,
    26150,
    27500,
    28950,
    30450,
    35150,
    // Galar
    4e4,
    42800,
    46150,
    49650,
    53350,
    64950,
    // Paldea
    75e3,
    84650,
    95950,
    108e3,
    120550,
    160300
  ]);
  var PORTAS_REGIAO = Object.freeze([
    { regiao: "paldea", nivel: 75e3 },
    { regiao: "galar", nivel: 4e4 },
    { regiao: "alola", nivel: 25e3 },
    { regiao: "kalos", nivel: 1e4 },
    { regiao: "unova", nivel: NIVEL_MIN_FRAGMENTO_OVO_SHINY }
  ]);
  function degrauFragmentoOvoShiny(nivelHunt) {
    const n = Number(nivelHunt);
    if (!Number.isFinite(n) || n < NIVEL_MIN_FRAGMENTO_OVO_SHINY) return 0;
    let d = 0;
    for (const nivel of DEGRAUS_FRAGMENTO_OVO_SHINY) {
      if (nivel > n) break;
      d++;
    }
    return d;
  }
  function chanceFragmentoOvoShiny(hunt) {
    if (!hunt || hunt.area === "outland") return 0;
    return chanceDoDegrauOvoShiny(degrauFragmentoOvoShiny(hunt.nivel));
  }
  function chanceDoDegrauOvoShiny(degrau) {
    return degrau > 0 ? CHANCE_FRAGMENTO_OVO_SHINY_BASE + (degrau - 1) * CHANCE_FRAGMENTO_OVO_SHINY_PASSO : 0;
  }
  function regiaoDoNivelOvoShiny(nivelHunt) {
    const n = Number(nivelHunt);
    return PORTAS_REGIAO.find((p) => n >= p.nivel)?.regiao ?? null;
  }
  function tabelaFragmentoOvoShiny() {
    return DEGRAUS_FRAGMENTO_OVO_SHINY.map((nivel, i) => ({
      nivel,
      degrau: i + 1,
      chance: chanceDoDegrauOvoShiny(i + 1),
      regiao: regiaoDoNivelOvoShiny(nivel)
    }));
  }

  // tests/.cache/shared/estado-delta.mjs
  function criarMescladorDeEstado() {
    let campos = null;
    let pokemons = /* @__PURE__ */ new Map();
    return function mesclar(parcial) {
      if (!parcial) return campos ? montar() : null;
      const { cheio, pokemons: lista, pkMud, pkFora, dexMud, dexFora, ...resto } = parcial;
      if (cheio || !campos) {
        campos = resto;
        pokemons = new Map((lista ?? []).map((k) => [k.id, k]));
        return montar();
      }
      Object.assign(campos, resto);
      if (lista) pokemons = new Map(lista.map((k) => [k.id, k]));
      for (const k of pkMud ?? []) pokemons.set(k.id, k);
      for (const id of pkFora ?? []) pokemons.delete(id);
      if (dexMud || dexFora) {
        campos.pokedex = { ...campos.pokedex ?? {}, ...dexMud ?? {} };
        for (const chave of dexFora ?? []) delete campos.pokedex[chave];
      }
      return montar();
    };
    function montar() {
      return { ...campos, pokemons: [...pokemons.values()] };
    }
  }

  // <stdin>
  var mod = { cooldown: cooldown_golpes_exports, especiais: golpes_especiais_exports, planilhas: golpes_planilhas_exports, outland: outland_exports, baseOutland: herdar_base_outland_exports, golpesOrre: herdar_golpes_orre_exports, megas: megas_exports, evolucoes: evolucoes_cruzadas_exports, recompensa: recompensa_hunt_exports, ajustes: ajustes_especie_espelho_exports, analyser: hunt_analyser_exports, ginasios: ginasios_exports, sazonal: dia_sazonal_exports, captura: captura_degrau_exports, ovoShiny: ovo_shiny_exports };
  (globalThis.__pokeidleHunt ?? (globalThis.__pokeidleHunt = {})).definirModulosDoJogo?.(mod, criarMescladorDeEstado);
})();
