#!/usr/bin/env node
// Generates the Harbor Market benchmark dataset: products.json, categories.json and orders.json.
//
// The output is fully deterministic (seeded PRNG, no dependencies, no clock access), so running
// `node generate.mjs` again reproduces the checked-in files byte for byte.

import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const OUT_DIR = dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------------------------
// Random helpers

function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const rand = mulberry32(0x48a2b0e7);
const int = (min, max) => min + Math.floor(rand() * (max - min + 1));
const pick = (list) => list[Math.floor(rand() * list.length)];
const chance = (p) => rand() < p;

function shuffle(list) {
  const out = list.slice();
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

function sample(list, n) {
  return shuffle(list).slice(0, n);
}

function weighted(entries) {
  const total = entries.reduce((sum, [, w]) => sum + w, 0);
  let r = rand() * total;
  for (const [value, w] of entries) {
    r -= w;
    if (r < 0) return value;
  }
  return entries[entries.length - 1][0];
}

// Fills `{key}` placeholders.
function fill(template, vars) {
  return template.replace(/\{(\w+)\}/g, (_, key) => {
    if (!(key in vars)) throw new Error(`missing template variable ${key}`);
    return vars[key];
  });
}

// ---------------------------------------------------------------------------------------------
// Dates (all UTC, computed from epoch days so no time zone is involved)

const DAY_MS = 86_400_000;
const dayOf = (iso) => Math.floor(Date.parse(`${iso}T00:00:00Z`) / DAY_MS);
const isoDate = (day) => new Date(day * DAY_MS).toISOString().slice(0, 10);
const isoDateTime = (seconds) => new Date(seconds * 1000).toISOString().replace(".000Z", "Z");

const CATALOG_START = dayOf("2024-01-08");
const CATALOG_END = dayOf("2026-09-30");
const DATASET_END = dayOf("2026-10-31");

// ---------------------------------------------------------------------------------------------
// Catalog content

const COLORS = ["charcoal", "sand", "sage", "navy", "terracotta", "ochre", "plum", "sky"];
const COLOR_LABELS = {
  charcoal: "Charcoal",
  sand: "Sand",
  sage: "Sage",
  navy: "Navy",
  terracotta: "Terracotta",
  ochre: "Ochre",
  plum: "Plum",
  sky: "Sky",
};

const ORIGINS = ["Portugal", "Japan", "Vietnam", "Germany", "United States", "Taiwan", "Türkiye", "Italy", "Canada", "Sweden"];
const WARRANTIES = ["1 year", "2 years", "2 years", "3 years", "5 years", "Lifetime"];

// Each category lists its product bases with their taglines, and the series names they are sold
// under. Every base is combined with distinct series names until the category reaches `count`
// products.
const CATEGORIES = [
  {
    slug: "outdoor",
    kind: "camping gear",
    name: "Outdoor & Camping",
    description: "Tents, packs, stoves and the small things that make a night under the stars feel easy.",
    count: 32,
    price: [19, 489],
    series: ["Ridgeline", "Tidewater", "Northshore", "Switchback"],
    bases: [
      ["2-Person Backpacking Tent", "Sets up in under 3 minutes, even in the dark."],
      ["800-Fill Down Sleeping Bag", "Rated for nights down to -5 °C."],
      ["Carbon Trekking Poles", "Weighs less than your phone & charger."],
      ["Rechargeable Headlamp", "400 lumens & a red mode that won't wake the tent."],
      ["Titanium Camp Stove", "Boils a liter of water in < 4 minutes."],
      ["Squeeze Water Filter", "Removes 99.9999% of bacteria; 1,500 L per cartridge."],
      ["Ultralight Daypack", "20 L, 380 g & a pocket for everything."],
      ["Hammock & Strap Kit", "Hangs between any two trees in under a minute."],
      ["Insulated Sleeping Pad", "R-value 4.2: warm enough for the shoulder seasons."],
      ["Folding Camp Chair", "Folds down smaller than a water bottle."],
      ["Packable Rain Shell", "Fully taped seams; packs into its own pocket."],
      ["Camp Lantern", "Warm light for 60 hours on one charge."],
      ["Roll-Top Dry Bag", "Waterproof to 1 m, even when you're not."],
      ["Enamel Camp Mug", "The \"just one more coffee\" mug."],
      ["Firestarter Kit", "Lights in wind, rain & snow."],
      ["Trail First-Aid Kit", "Packed by wilderness medics for 1-4 people."],
    ],
    materials: ["ripstop nylon (20D)", "recycled polyester", "aluminium 7001", "titanium", "silicone-coated nylon", "waxed cotton canvas"],
    uses: ["thru-hikes", "car camping", "overnighters", "alpine starts", "canoe trips", "festival weekends"],
    features: [
      "Every buckle, zip and toggle works with gloves on.",
      "It is repairable in the field, and spare parts are always in stock.",
      "Reflective details catch your headlamp when you're hunting for it at 2 a.m.",
      "It packs into its own stuff sack, so nothing rattles around in your pack.",
      "Water-repellent finishes are PFC-free.",
      "Our guides test every design for a full season before it goes on sale.",
      "It is light enough for long trails and tough enough for scrambles.",
      "Glow-in-the-dark pulls make it easy to find in a dark tent.",
    ],
    specs: [
      ["Weight", () => `${int(80, 2400)} g`],
      ["Packed size", () => `${int(8, 22)} × ${int(10, 45)} cm`],
      ["Temperature rating", () => `${int(-9, 9)} °C`],
      ["Water column", () => `${int(15, 50) * 100} mm`],
    ],
  },
  {
    slug: "kitchen",
    kind: "kitchenware",
    name: "Kitchen & Dining",
    description: "Knives, cookware and tableware for people who cook every day, not just on Sundays.",
    count: 24,
    price: [14, 349],
    series: ["Galley", "Hearth", "Saltworks", "Larder"],
    bases: [
      ["Chef's Knife", "Hand-finished in a small workshop in Seki, Japan."],
      ["Cast Iron Skillet", "Pre-seasoned & ready for the stovetop, oven or campfire."],
      ["Salt & Pepper Mill Set", "Ceramic burrs that never rust or dull."],
      ["Pour-Over Coffee Kit", "From the roasters at Saltworks: \"the one we actually use.\""],
      ["Crème Brûlée Torch", "Caramelizes sugar in seconds; refills with any butane."],
      ["Linen Apron", "Pre-washed European linen with deep pockets."],
      ["End-Grain Cutting Board", "Kind to knives, tough on everything else."],
      ["Stoneware Dinner Plates (Set of 4)", "Oven, microwave & dishwasher safe."],
      ["Café Espresso Cups", "Thick walls keep a double shot hot."],
      ["Enamelled Dutch Oven", "Oven safe to 260 °C."],
      ["Digital Kitchen Scale", "Accurate to 0.1 g for coffee, bread & everything between."],
      ["Glass Storage Containers", "Stacks neatly in cupboards < 30 cm deep."],
    ],
    materials: ["high-carbon steel", "cast iron", "stoneware", "European linen", "end-grain walnut", "borosilicate glass"],
    uses: ["weeknight dinners", "slow Sunday braises", "dinner parties", "morning coffee", "meal prep", "holiday baking"],
    features: [
      "It is comfortable to use through long prep sessions, in large and small hands alike.",
      "It cleans up quickly, so it actually gets used every day.",
      "The finish resists scratches from everyday use.",
      "It looks as good on the table as it works in the kitchen.",
      "Each piece is inspected by hand before it leaves the workshop.",
      "Food-safe materials mean nothing leaches into your dinner.",
      "It is built to be handed down, not thrown out.",
      "It stores compactly in a crowded drawer or cupboard.",
    ],
    specs: [
      ["Capacity", () => `${(int(4, 60) / 10).toFixed(1)} L`],
      ["Dishwasher safe", () => pick(["Yes", "No", "Top rack only"])],
      ["Oven safe", () => pick(["Up to 230 °C", "Up to 260 °C", "No"])],
      ["Weight", () => `${(int(2, 45) / 10).toFixed(1)} kg`],
    ],
  },
  {
    slug: "home",
    kind: "homeware",
    name: "Home & Living",
    description: "Soft textiles, warm light and quiet storage for rooms you want to spend time in.",
    count: 22,
    price: [12, 420],
    series: ["Moorings", "Linden", "Quayside", "Fjord"],
    bases: [
      ["Wool Throw Blanket", "The \"just one more\" blanket everybody fights over."],
      ["Linen Duvet Cover", "Gets softer with every wash."],
      ["Ceramic Table Lamp", "Warm light for long evenings & lazy mornings."],
      ["Scented Soy Candle", "Burns cleanly for 60+ hours."],
      ["Woven Storage Basket", "Hand-woven seagrass with sturdy rope handles."],
      ["Oak Wall Shelf", "Holds up to 25 kg once it's on the wall."],
      ["Jute Area Rug", "Woven on traditional looms in Guimarães, Portugal."],
      ["Stoneware Vase", "Thrown by hand, so no two are quite the same."],
      ["Turkish Cotton Towel Set", "Light, quick-drying & surprisingly absorbent."],
      ["Brass Wall Hooks (Set of 3)", "Solid brass that ages to a warm patina."],
      ["Blackout Curtains", "Blocks 99% of light, so weekends start when you say so."],
    ],
    materials: ["merino wool", "stonewashed linen", "solid oak", "hand-thrown ceramic", "natural jute", "Turkish cotton"],
    uses: ["reading nooks", "guest rooms", "small apartments", "cosy evenings", "entryways", "bedrooms"],
    features: [
      "Natural materials breathe, so it feels right in summer and winter.",
      "Every piece varies slightly in tone, which is part of the charm.",
      "Everything you need to set it up is in the box.",
      "It is finished with a plant-based oil instead of lacquer.",
      "Colors come from low-impact, OEKO-TEX certified dyes.",
      "It arrives in plastic-free, recyclable packaging.",
      "Seams and edges are finished by hand.",
      "It's designed to be repaired, not replaced.",
    ],
    specs: [
      ["Dimensions", () => `${int(20, 220)} × ${int(20, 200)} cm`],
      ["Care", () => pick(["Machine wash cold, line dry", "Wipe clean with a damp cloth", "Dry clean only", "Machine wash 40 °C"])],
      ["Weight", () => `${(int(2, 90) / 10).toFixed(1)} kg`],
    ],
  },
  {
    slug: "electronics",
    kind: "tech",
    name: "Electronics",
    description: "Chargers, cameras and gadgets that quietly do their job and get out of the way.",
    count: 22,
    price: [15, 399],
    series: ["Volt", "Beacon", "Lumen", "Relay"],
    bases: [
      ["65W USB-C Charger", "One charger for your laptop, phone & earbuds."],
      ["Wireless Charging Pad", "Charges through cases up to 5 mm thick."],
      ["20,000 mAh Power Bank", "Fits in a jeans pocket; charges a laptop."],
      ["Smart Plug (2-Pack)", "Works with iOS, Android & most smart-home hubs."],
      ["4K Webcam", "Looks great on calls, even in a dim room."],
      ["E-Ink Reader", "Battery life you'll measure in weeks, not hours."],
      ["Bluetooth Tracker Tag", "Pair it once, clip it to your keys, stop searching."],
      ["Universal Travel Adapter", "Certified for 160+ countries."],
      ["Foldable Solar Charger", "Charges a phone to 50% in < 3 hours of sun."],
      ["Waterproof Action Camera", "Waterproof to 10 m without a case."],
      ["Smart Thermostat", "Learns your schedule in about a week."],
    ],
    materials: ["anodized aluminium", "recycled polycarbonate", "Gorilla Glass", "silicone", "gallium nitride (GaN)", "magnesium alloy"],
    uses: ["travel days", "home offices", "road trips", "long flights", "video calls", "weekend projects"],
    features: [
      "It works out of the box, with no account or cloud subscription required.",
      "Firmware updates arrive over the air through the companion app.",
      "A braided USB-C cable is included in the box.",
      "Protection against overheating and power surges is built in.",
      "The status light dims automatically at night.",
      "The matte finish shrugs off fingerprints.",
      "It is designed to be repaired, with a replaceable battery where it has one.",
      "We guarantee at least five years of firmware updates.",
    ],
    specs: [
      ["Battery", () => pick(["None (mains powered)", `${int(5, 200) * 100} mAh`, `${int(2, 40) * 10} hours`])],
      ["Connectivity", () => pick(["USB-C, Bluetooth 5.3", "Wi-Fi 6, Bluetooth 5.2", "USB-C PD 3.1", "Bluetooth LE", "Wi-Fi 2.4 GHz"])],
      ["Input", () => pick(["100-240 V AC", "USB-C 5 V / 3 A", "USB-C 20 V / 3.25 A"])],
      ["Weight", () => `${int(18, 650)} g`],
    ],
  },
  {
    slug: "apparel",
    kind: "clothing",
    name: "Apparel",
    description: "Layers that work as well on a ferry deck as they do in the office.",
    count: 20,
    price: [18, 329],
    series: ["Ebbtide", "Mariner", "Crofter", "Saltmarsh"],
    bases: [
      ["Merino Base Layer", "Naturally odor-resistant, so you can pack less."],
      ["Fleece Half-Zip", "Soft enough to sleep in, tough enough for the trail."],
      ["Organic Cotton Tee", "The \"wear it every day\" one."],
      ["Waxed Canvas Jacket", "Re-waxable & made to last a decade or two."],
      ["Ribbed Wool Beanie", "Knitted in Donegal from undyed wool."],
      ["Stretch Chino Trousers", "Cut a little higher in the back for bike commutes."],
      ["Packable Down Vest", "Packs into its own pocket & weighs 210 g."],
      ["Brushed Flannel Shirt", "Size up if you like to layer underneath."],
      ["UPF 50+ Sun Hoodie", "Sun protection without the sticky sunscreen."],
      ["Fisherman's Rib Sweater", "A chunky knit for cold decks & warm pubs."],
    ],
    materials: ["18.5 µm merino wool", "organic cotton", "recycled polyester fleece", "waxed cotton canvas", "Responsible Down (RDS)", "cotton-hemp blend"],
    uses: ["daily commutes", "cold mornings", "travel", "trail days", "weekends away", "layering"],
    features: [
      "Flatlock seams sit flat against the skin and never chafe.",
      "It is cut to move with you, on a bike, a boat or a mountain.",
      "The fabric is pre-shrunk, so the fit you buy is the fit you keep.",
      "Natural fibers regulate temperature, so it is comfortable across seasons.",
      "It is sewn in a Fair Trade certified factory.",
      "Reinforced stress points hold up to years of daily wear.",
      "It looks just as good after fifty washes as it did after one.",
      "Colors come from low-impact, bluesign® approved dyes.",
    ],
    specs: [
      ["Fit", () => pick(["Regular", "Relaxed", "Slim", "Oversized"])],
      ["Sizes", () => pick(["XS-XXL", "S-XL", "XS-3XL", "One size"])],
      ["Care", () => pick(["Machine wash cold, lay flat to dry", "Hand wash only", "Machine wash 30 °C", "Spot clean, re-wax yearly"])],
    ],
  },
  {
    slug: "footwear",
    kind: "footwear",
    name: "Footwear",
    description: "Boots, runners and slippers built on comfortable lasts and real rubber.",
    count: 18,
    price: [16, 289],
    series: ["Strand", "Scree", "Boardwalk", "Tor"],
    bases: [
      ["Trail Runner", "Grippy on wet rock, comfortable on pavement."],
      ["Waterproof Hiking Boot", "Waterproof, breathable & resoleable."],
      ["Leather Chelsea Boot", "Goodyear-welted so a cobbler can resole it."],
      ["Canvas Sneaker", "Broken in straight out of the box."],
      ["Shearling Camp Slipper", "A real rubber sole for trips to the mailbox."],
      ["Kids' Rain Boot", "Puddle-proof up to the knee, kid-proof all the way."],
      ["Recovery Sandal", "Runs half a size large; order down if you're between sizes."],
      ["Merino Hiking Socks (3-Pack)", "Cushioned where it matters, thin where it doesn't."],
      ["Approach Shoe", "Vibram® outsole with sticky climbing rubber."],
    ],
    materials: ["full-grain leather", "recycled mesh", "natural rubber", "suede", "merino wool", "organic cotton canvas"],
    uses: ["long hikes", "city walking", "rainy commutes", "rest days", "scrambles", "school runs"],
    features: [
      "A cushioned footbed softens long days on hard ground.",
      "It is built on a last with a roomy toe box.",
      "Removable insoles leave room for custom orthotics.",
      "Breathable materials keep feet dry from morning to night.",
      "It is lighter than it looks.",
      "Every design is wear-tested for 500 km before we sign off.",
      "Our repair program keeps your pair going for years.",
      "Grippy rubber holds on wet decks and slick pavements.",
    ],
    specs: [
      ["Sizes", () => pick(["EU 36-47", "EU 38-48", "EU 25-35 (kids)", "S/M/L"])],
      ["Drop", () => `${int(0, 12)} mm`],
      ["Weight (per shoe)", () => `${int(90, 720)} g`],
    ],
  },
  {
    slug: "bags",
    kind: "luggage",
    name: "Bags & Travel",
    description: "Carry-ons, packs and organizers that make getting out the door the easy part.",
    count: 18,
    price: [15, 379],
    series: ["Wayfarer", "Portside", "Transit", "Compass"],
    bases: [
      ["Weekender Duffel", "Holds a weekend's worth of clothes, < 1.2 kg empty."],
      ["30 L Laptop Backpack", "Fits laptops up to 16\" & most tablets."],
      ["Packing Cubes (Set of 3)", "Compresses your clothes by up to 40%."],
      ["Everyday Sling", "Hidden pockets for passports, cards & keys."],
      ["Carry-On Roller", "Carry-on approved on 90% of airlines."],
      ["Waxed Canvas Tote", "Made to be overstuffed."],
      ["Hanging Toiletry Kit", "Hangs on any hook & opens flat on the counter."],
      ["Camera Insert Bag", "Turns any backpack into a padded camera bag."],
      ["Bits & Bobs Organizer", "A place for every cable, charger & spare SIM."],
    ],
    materials: ["recycled ballistic nylon", "waxed cotton canvas", "full-grain leather", "X-Pac sailcloth", "polycarbonate shell", "Cordura® 500D"],
    uses: ["weekend trips", "daily commutes", "long-haul flights", "photo walks", "business travel", "gym days"],
    features: [
      "YKK AquaGuard® zips keep the contents dry in a downpour.",
      "Clever pockets keep small things where you can find them.",
      "Bar-tack stitching reinforces every stress point.",
      "Nothing snags on a conveyor belt or a car door.",
      "The base is waterproof and wipe-clean.",
      "It is made from recycled fabrics without compromising on strength.",
      "It fits under an airline seat or in an overhead bin.",
      "It is covered by our no-questions-asked lifetime repair promise.",
    ],
    specs: [
      ["Capacity", () => `${int(3, 60)} L`],
      ["Dimensions", () => `${int(20, 56)} × ${int(15, 40)} × ${int(5, 25)} cm`],
      ["Laptop sleeve", () => pick(["Fits up to 13\"", "Fits up to 16\"", "No"])],
      ["Weight", () => `${(int(2, 38) / 10).toFixed(1)} kg`],
    ],
  },
  {
    slug: "office",
    kind: "desk gear",
    name: "Office & Desk",
    description: "Tools for writing, thinking and working comfortably, at home or anywhere else.",
    count: 18,
    price: [9, 259],
    series: ["Ledger", "Draft", "Atlas", "Margin"],
    bases: [
      ["LED Desk Lamp", "Flicker-free light with 5 color temperatures."],
      ["Walnut Monitor Stand", "Raises your screen to eye level & tidies the cables."],
      ["A5 Dot Grid Notebook", "Fountain-pen friendly 100 g/m² paper."],
      ["Brass Fountain Pen", "Writes smoothly from the first stroke to the last page."],
      ["Felt Desk Mat", "Desk chaos -> desk calm in about 10 minutes."],
      ["Cable Organizer", "Keeps chargers within reach & off the floor."],
      ["Mechanical Keyboard", "Hot-swappable switches & a solid aluminium case."],
      ["Ergonomic Mouse", "Your wrist will thank you by Friday."],
      ["Leather Pencil Case", "Vegetable-tanned leather that softens with use."],
    ],
    materials: ["solid walnut", "brass", "merino felt", "recycled aluminium", "vegetable-tanned leather", "acid-free paper"],
    uses: ["deep work", "journaling", "video calls", "late-night sessions", "sketching", "small desks"],
    features: [
      "It is designed to make long days at a desk a little more comfortable.",
      "It stays where you put it, even on a smooth desk.",
      "Cables and clutter stay neatly out of sight.",
      "It is tested for years of daily use before it reaches your desk.",
      "Materials are chosen to age well rather than look worn.",
      "The finish resists coffee rings and pen marks.",
      "Everything is replaceable, down to the smallest part.",
      "It looks at home in an office, a studio or on a kitchen table.",
    ],
    specs: [
      ["Dimensions", () => `${int(10, 90)} × ${int(5, 40)} cm`],
      ["Compatibility", () => pick(["Universal", "Windows, macOS, Linux", "Standard #6 nibs", "Monitors up to 32\""])],
      ["Weight", () => `${int(40, 2200)} g`],
    ],
  },
  {
    slug: "garden",
    kind: "garden tools",
    name: "Garden",
    description: "Hard-wearing tools and simple kits for balconies, allotments and back gardens.",
    count: 18,
    price: [8, 299],
    series: ["Allotment", "Potting Shed", "Hedgerow", "Orchard"],
    bases: [
      ["Bypass Pruning Shears", "Cuts branches up to 20 mm & stays sharp all season."],
      ["Galvanized Watering Can", "Balanced for long watering rounds without the wrist ache."],
      ["Cedar Raised Garden Bed", "Untreated cedar that's naturally rot-resistant."],
      ["Kitchen Herb Seed Collection", "Germination rate > 85%, or we'll send new seeds."],
      ["Leather Garden Gloves", "Thorn-proof & still flexible enough for fiddly jobs."],
      ["Wall-Mounted Hose Reel", "Holds 30 m of hose & rewinds without kinks."],
      ["Copper Bird Feeder", "Squirrel-resistant (we tested it on Gerald)."],
      ["Compost Bin", "Turns kitchen scraps into compost in about 8 weeks."],
      ["Solar String Lights", "Charges by day, glows for 8 hours at night."],
    ],
    materials: ["Japanese SK5 steel", "galvanized steel", "Western red cedar", "goatskin leather", "recycled HDPE", "solid copper"],
    uses: ["balcony gardens", "allotments", "spring planting", "autumn tidying", "patio evenings", "first-time gardeners"],
    features: [
      "Replacement parts are available for years to come.",
      "It ships flat and assembles without tools.",
      "The finish is weatherproof, so it can stay outside all year.",
      "It comes with a planting calendar for your region.",
      "It is made from recycled or responsibly sourced materials.",
      "It is sized for balconies as well as back gardens.",
      "It's designed with wildlife-friendly gardening in mind.",
      "It stays comfortable through a long afternoon of jobs.",
    ],
    specs: [
      ["Dimensions", () => `${int(15, 180)} × ${int(10, 90)} cm`],
      ["Weatherproof", () => pick(["Yes", "Yes (IP65)", "Store indoors in winter"])],
      ["Weight", () => `${(int(1, 160) / 10).toFixed(1)} kg`],
    ],
  },
  {
    slug: "audio",
    kind: "audio gear",
    name: "Audio",
    description: "Speakers, headphones and turntables tuned for listening, not just for spec sheets.",
    count: 16,
    price: [29, 699],
    series: ["Sonar", "Echo Bay", "Harmonic", "Fathom"],
    bases: [
      ["Wireless Earbuds", "Active noise cancelling that doesn't hiss."],
      ["Over-Ear Headphones", "36 hours of playback on a single charge."],
      ["Waterproof Bluetooth Speaker", "IP67: dust-tight & happy in the pool."],
      ["Belt-Drive Turntable", "Plays your records the way they were cut."],
      ["Bookshelf Speakers (Pair)", "\"The best-sounding thing on my desk\" - our head of sound."],
      ["Portable DAB+ Radio", "Warm, room-filling sound from a radio < 1 kg."],
      ["Compact Soundbar", "Clear dialogue, even at whisper-quiet volumes."],
      ["USB Studio Microphone", "Broadcast-quality voice for podcasts & calls."],
    ],
    materials: ["anodized aluminium", "walnut veneer", "recycled ABS", "protein leather", "woven fabric", "steel mesh"],
    uses: ["morning commutes", "dinner parties", "record nights", "podcasting", "beach days", "focus sessions"],
    features: [
      "Custom-tuned drivers deliver a balanced, natural sound.",
      "Physical controls work even with wet or cold fingers.",
      "The companion app includes a 5-band EQ.",
      "Auto-standby saves power when nothing is playing.",
      "It is tuned by ear in our Portland listening room.",
      "Replaceable parts extend its life for years.",
      "It connects in seconds and remembers your devices.",
      "It supports high-resolution codecs wherever your source does.",
    ],
    specs: [
      ["Battery life", () => pick(["None (mains powered)", `${int(6, 60)} hours`])],
      ["Frequency response", () => `${int(15, 50)} Hz - ${int(18, 40)} kHz`],
      ["Connectivity", () => pick(["Bluetooth 5.3", "Bluetooth 5.3, 3.5 mm", "USB-C, XLR", "RCA, Bluetooth", "HDMI eARC, optical"])],
      ["Weight", () => `${int(5, 4800)} g`],
    ],
  },
  {
    slug: "fitness",
    kind: "fitness gear",
    name: "Fitness",
    description: "Gear for home workouts, long runs and everything in between.",
    count: 16,
    price: [9, 549],
    series: ["Tempo", "Breakwater", "Stride", "Current"],
    bases: [
      ["Natural Rubber Yoga Mat", "Non-slip, even in the sweatiest hot-yoga class."],
      ["Adjustable Dumbbells (Pair)", "Replaces 15 pairs of dumbbells & fits under the bed."],
      ["Resistance Band Set", "Five levels, from \"warm-up\" to \"why?\""],
      ["High-Density Foam Roller", "Gentle on joints, tough on knots."],
      ["Weighted Jump Rope", "Tangle-free bearings for double-unders & beyond."],
      ["Cast Iron Kettlebell", "Powder-coated for a grip that doesn't need chalk."],
      ["Insulated Water Bottle (750 ml)", "Ice-cold for 24 hours, hot for 12."],
      ["Fitness Tracker", "Tracks heart rate, sleep & recovery without a subscription."],
    ],
    materials: ["natural tree rubber", "cast iron", "natural latex", "EVA foam", "18/8 stainless steel", "steel cable"],
    uses: ["home workouts", "morning routines", "recovery days", "small spaces", "long runs", "travel training"],
    features: [
      "It is sized to fit in a small apartment without becoming furniture.",
      "Free guided workouts come with every purchase.",
      "The surface is easy to wipe down after a sweaty session.",
      "Clear markings make progress easy to track.",
      "It's tested well beyond its rated load.",
      "It is easy to take outside on sunny days.",
      "Sweat-proof and splash-resistant from top to bottom.",
      "Everything ships in a single box with no plastic.",
    ],
    specs: [
      ["Weight", () => `${(int(1, 240) / 10).toFixed(1)} kg`],
      ["Dimensions", () => `${int(10, 183)} × ${int(5, 66)} cm`],
      ["Max load", () => pick(["N/A", `${int(5, 30) * 10} kg`])],
    ],
  },
  {
    slug: "toys",
    kind: "toys",
    name: "Toys & Games",
    description: "Wooden toys, puzzles and games for curious kids and competitive grown-ups.",
    count: 16,
    price: [9, 189],
    series: ["Lighthouse", "Rockpool", "Seagull", "Tugboat"],
    bases: [
      ["Wooden Building Blocks", "Ages 3+ & certified to EN 71."],
      ["1,000-Piece Jigsaw Puzzle", "The puzzle everyone \"just helps with for a minute.\""],
      ["Stunt Kite", "Flies in winds from 8 to 40 km/h."],
      ["Board Game: \"Harbor Lights\"", "2-6 players, 30-45 minutes, endless rematches."],
      ["Plush Sea Otter", "Soft, washable & surprisingly huggable."],
      ["Marble Run Starter Set", "Educational, though nobody has to know that."],
      ["Junior Telescope", "Sees Saturn's rings on a clear night."],
      ["Card Game: Tides & Tales", "Learn it in 5 minutes, play it for years."],
    ],
    materials: ["FSC-certified beech", "recycled cardboard", "ripstop nylon", "organic cotton plush", "optical glass", "bioplastic"],
    uses: ["rainy afternoons", "family game nights", "birthday gifts", "beach days", "stargazing", "quiet time"],
    features: [
      "All the edges are rounded and sanded smooth by hand.",
      "Pieces store neatly in the box, which doubles as a tray.",
      "Instructions are short, so you can start playing in minutes.",
      "It's tested by a panel of very honest seven-year-olds.",
      "Replacement parts are available if something goes missing.",
      "Bright, non-toxic colors are safe for little mouths.",
      "It grows with your child, with harder challenges included.",
      "It packs away neatly when play is done.",
    ],
    specs: [
      ["Ages", () => pick(["3+", "6+", "8+", "10+", "14+"])],
      ["Players", () => pick(["1", "1-4", "2-6", "2+"])],
      ["Safety", () => pick(["EN 71, ASTM F963", "EN 71", "CE, UKCA"])],
    ],
  },
];

const INTROS = [
  "Meet the {name}, our answer to the question we kept hearing: why is it so hard to find {kind} that simply works?",
  "The {name} started as a sketch on the back of a ferry timetable and went through eleven prototypes before we were happy.",
  "We designed the {name} for {use}, and then kept using it everywhere else too.",
  "If you've ever been let down by cheap {kind}, the {name} is for you.",
  "The {name} is the kind of thing you buy once and stop thinking about.",
  "Built from {material}, the {name} is made to be used hard and handed down.",
];

const MIDDLES = [
  "It is made from {material}, chosen because it ages well and is easy to care for.",
  "We worked with a small, family-run factory in {origin} that has been making {kind} for three generations.",
  "Testers took it on {use} for six months and sent back pages of notes, most of which ended up in the final design.",
  "Every detail was questioned: if it didn't earn its place, it went.",
  "The {color} colorway was mixed to match the harbor at dusk.",
  "It is equally at home on {use} and on a quiet weekday at home.",
];

const CLOSERS = [
  "It's covered by our {warranty_phrase} warranty, and if anything breaks we'll repair it or replace it.",
  "Try it for 30 days; if it isn't right, send it back and we'll cover the return shipping.",
  "Every order ships in recycled packaging from our warehouse in Portland, usually within one business day.",
  "We think it is the best {kind} you can buy for the price, and our customers seem to agree.",
  "Questions about sizing or fit? Our support team replies within a day, in English, Deutsch or Español.",
];

// ---------------------------------------------------------------------------------------------
// Reviews

const REVIEWERS = [
  "Zoë M.", "José G.", "Seán O.", "Mei L.", "Priya K.", "Łukasz W.", "Ngozi A.", "Björn S.",
  "Aoife D.", "Mateo R.", "Hana T.", "François B.", "Emily C.", "Kwame O.", "Sofia P.", "Liam H.",
  "Anaïs R.", "Tomás F.", "Yuki N.", "Grace W.", "Dmitri V.", "Chloé L.", "Noah J.", "Amara E.",
  "Henrik J.", "Isabel S.", "Rafael C.", "Jun P.", "Olivia K.", "Ahmed Z.", "Maya R.", "Ben O'C.",
];

const REVIEW_TITLES = {
  5: ["Exactly what I wanted", "Worth every penny", "Best purchase this year", "Five stars & no regrets", "Better than expected", "Bought a second one", "Built like a tank", "Love it"],
  4: ["Really good, one small gripe", "Great quality", "Very happy overall", "Solid & well made", "Would buy again", "Nearly perfect"],
  3: ["Does the job", "Good, not great", "Mixed feelings", "Fine for the price", "OK once you get used to it"],
  2: ["Not for me", "Expected more", "Disappointing finish", "Quality < price"],
  1: ["Broke after a week", "Returned it", "Would not recommend"],
};

const REVIEW_BODIES = {
  5: [
    "I've used it on {use} for three months now and it still looks new. Fit & finish are excellent.",
    "My partner called it \"the good one\" within a day, and now I have to buy another.",
    "Setup took < 5 minutes. It feels far more expensive than it was.",
    "Replaced a cheaper {kind} that lasted one season. This one is clearly built to last.",
    "Arrived two days early, beautifully packed, and the {color} color is even nicer in person.",
    "Honestly can't fault it. It does exactly what it says & nothing gets in the way.",
    "Used it every day since it arrived. Quality > hype, for once.",
  ],
  4: [
    "Really well made. I'd give it five stars if it came in more colors.",
    "Great for {use}. It's slightly heavier than I expected, but that's probably why it feels so sturdy.",
    "Does everything I need. The instructions could be clearer, but I figured it out.",
    "Very good value. Took a week to get used to it, now I wouldn't go back.",
    "Lovely {material}. Mine had a tiny mark on one side but support sent a replacement without fuss.",
  ],
  3: [
    "It's fine. Not as premium as the photos suggest, but it works for {use}.",
    "Decent, but I expected a bit more for the price. Customer service was great though.",
    "Works as described. The {color} is a little darker than on screen.",
    "Good idea, average execution. I'll keep it, but I'm not raving about it.",
  ],
  2: [
    "The stitching started to come loose after a few weeks of {use}.",
    "It works, but it feels flimsy & I'm not sure it'll last a year.",
    "Smaller than I expected. Check the dimensions before ordering.",
  ],
  1: [
    "Stopped working after a week. Return process was easy, at least.",
    "Not what I expected at all; sent it straight back.",
  ],
};

// Strings that probe HTML escaping. One is attached to a few fixed products after ids are assigned.
const ESCAPE_REVIEWS = [
  {
    author: "<script>alert('hi')</script>",
    rating: 5,
    title: "Tried to break the review form",
    body: "I typed <script>alert(\"pwned\")</script> into the review box & it just showed up as text. Also the product is great.",
  },
  {
    author: "Ana & Rui",
    rating: 4,
    title: "\"Sturdy\" is an understatement",
    body: "We compared it with three others: price < theirs, quality > theirs. Only gripe: the box said 'easy assembly' & it wasn't.",
  },
];

// ---------------------------------------------------------------------------------------------
// Products

function priceCents(min, max) {
  const dollars = int(min, max);
  const cents = weighted([[0, 3], [99, 4], [50, 2], [95, 1]]);
  return dollars * 100 + cents;
}

function rating() {
  return weighted([
    [() => int(43, 49), 5],
    [() => int(36, 42), 4],
    [() => int(25, 35), 2],
    [() => int(10, 24), 1],
    [() => 50, 1],
  ])() / 10;
}

function reviewRating(productRating) {
  const r = Math.round(productRating + (rand() - 0.5) * 2.4);
  return Math.min(5, Math.max(1, r));
}

function buildDescription(category, vars) {
  const paragraphs = [];
  const features = shuffle(category.features);
  const middles = shuffle(MIDDLES);
  paragraphs.push([fill(pick(INTROS), vars), fill(middles[0], vars), features[0]].join(" "));
  paragraphs.push([features[1], fill(middles[1], vars), features[2], features[3]].join(" "));
  if (chance(0.6)) {
    paragraphs.push([features[4], fill(pick(CLOSERS), vars)].join(" "));
  }
  return paragraphs.join("\n\n");
}

function buildSpecs(category, vars) {
  const specs = [
    ["Material", vars.material[0].toUpperCase() + vars.material.slice(1)],
    ...category.specs.map(([label, value]) => [label, value()]),
    ["Color", vars.color],
    ["Country of origin", vars.origin],
    ["Warranty", vars.warranty],
  ];
  // 6 to 8 rows: drop optional rows from the middle if there are too many.
  const target = int(6, Math.min(8, specs.length));
  while (specs.length > target) specs.splice(1 + Math.floor(rand() * (specs.length - 4)), 1);
  return specs.map(([label, value]) => ({ label, value }));
}

function buildReviews(category, vars, productRating, createdDay) {
  const count = int(6, 10);
  const reviews = [];
  const authors = sample(REVIEWERS, count);
  for (let i = 0; i < count; i++) {
    const r = reviewRating(productRating);
    reviews.push({
      author: authors[i],
      rating: r,
      title: pick(REVIEW_TITLES[r]),
      body: fill(pick(REVIEW_BODIES[r]), vars),
      date: isoDate(int(Math.min(createdDay + 7, DATASET_END), DATASET_END)),
    });
  }
  // Newest review first.
  reviews.sort((a, b) => (a.date < b.date ? 1 : a.date > b.date ? -1 : 0));
  return reviews;
}

const drafts = [];
for (const category of CATEGORIES) {
  const names = new Set();
  let round = 0;
  while (names.size < category.count) {
    for (const [baseIndex, [base, tagline]] of category.bases.entries()) {
      if (names.size >= category.count) break;
      const series = category.series[(baseIndex + round) % category.series.length];
      const name = `${series} ${base}`;
      if (names.has(name)) continue;
      names.add(name);

      const color = pick(COLORS);
      const vars = {
        name,
        kind: category.kind,
        use: pick(category.uses),
        material: pick(category.materials),
        origin: pick(ORIGINS),
        warranty: pick(WARRANTIES),
        warranty_phrase: "",
        color: COLOR_LABELS[color],
      };
      vars.warranty_phrase = vars.warranty === "Lifetime" ? "lifetime" : vars.warranty.replace(/ years?$/, "-year");
      const createdDay = int(CATALOG_START, CATALOG_END);
      const productRating = rating();
      const price = priceCents(category.price[0], category.price[1]);
      drafts.push({
        name,
        category: category.slug,
        price_cents: price,
        compare_at_cents: chance(0.22) ? Math.max(price + 500, Math.round((price * (1.15 + rand() * 0.4)) / 100) * 100) - 1 : null,
        rating: productRating,
        review_count: weighted([[() => int(12, 180), 5], [() => int(180, 1400), 4], [() => int(1400, 6200), 1]])(),
        stock: weighted([[() => 0, 1], [() => int(1, 9), 1.5], [() => int(10, 240), 7.5]])(),
        color,
        tagline,
        description: buildDescription(category, vars),
        specs: buildSpecs(category, vars),
        created_at: isoDate(createdDay),
        featured: false,
        related: [],
        reviews: buildReviews(category, vars, productRating, createdDay),
      });
    }
    round++;
  }
}

function slugify(text) {
  return text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

const products = shuffle(drafts).map((draft, index) => {
  const id = index + 1;
  return {
    id,
    slug: slugify(draft.name),
    name: draft.name,
    category: draft.category,
    price_cents: draft.price_cents,
    compare_at_cents: draft.compare_at_cents,
    rating: draft.rating,
    review_count: draft.review_count,
    stock: draft.stock,
    color: draft.color,
    tagline: draft.tagline,
    description: draft.description,
    specs: draft.specs,
    created_at: draft.created_at,
    featured: false,
    related: [],
    reviews: draft.reviews,
  };
});

// Escape probes on fixed ids, so the rotating detail scenario always covers them.
products[0].reviews[0] = { ...ESCAPE_REVIEWS[0], date: products[0].reviews[0].date };
products[1].reviews[1] = { ...ESCAPE_REVIEWS[1], date: products[1].reviews[1].date };

// Related products: four others from the same category.
for (const product of products) {
  const peers = products.filter((p) => p.category === product.category && p.id !== product.id);
  product.related = sample(peers, 4)
    .map((p) => p.id)
    .sort((a, b) => a - b);
}

// Featured: eight in-stock, well-rated products from eight different categories.
{
  const featuredCategories = ["outdoor", "kitchen", "home", "apparel", "bags", "audio", "office", "electronics"];
  for (const slug of featuredCategories) {
    const candidates = products.filter((p) => p.category === slug && p.stock > 0 && p.rating >= 4.2);
    pick(candidates).featured = true;
  }
}

// Guarantee the edge cases the reference app documents.
{
  const featured = products.filter((p) => p.featured).length;
  const soldOut = products.filter((p) => p.stock === 0).length;
  const lowStock = products.filter((p) => p.stock > 0 && p.stock < 10).length;
  const onSale = products.filter((p) => p.compare_at_cents !== null).length;
  const outdoor = products.filter((p) => p.category === "outdoor").length;
  const fiveStar = products.filter((p) => p.rating === 5).length;
  if (featured !== 8 || soldOut === 0 || lowStock === 0 || onSale === 0 || outdoor <= 24 || fiveStar === 0) {
    throw new Error("dataset invariants not met");
  }
  if (new Set(products.map((p) => p.slug)).size !== products.length) throw new Error("duplicate slug");
}

// ---------------------------------------------------------------------------------------------
// Categories

const categories = CATEGORIES.map(({ slug, name, description }) => ({ slug, name, description }));

// ---------------------------------------------------------------------------------------------
// Orders

const FIRST_NAMES = [
  "Zoë", "José", "Seán", "Mei", "Priya", "Łukasz", "Ngozi", "Björn", "Aoife", "Mateo", "Hana",
  "François", "Emily", "Kwame", "Sofia", "Liam", "Anaïs", "Tomás", "Yuki", "Grace", "Dmitri",
  "Chloé", "Noah", "Amara", "Henrik", "Isabel", "Rafael", "Jun", "Olivia", "Ahmed", "Maya",
  "Ben", "Fatima", "Lars", "Elena", "Kofi", "Ingrid", "Diego", "Aiko", "Nadia",
];
const LAST_NAMES = [
  "Müller", "García", "O'Brien", "Lin", "Kapoor", "Wiśniewski", "Adeyemi", "Svensson", "Doyle",
  "Rossi", "Tanaka", "Bernard", "Clarke", "Owusu", "Papadopoulos", "Hughes", "Roux", "Fernández",
  "Nakamura", "Walker", "Volkov", "Lefèvre", "Johnson", "Eze", "Jensen", "Silva", "Costa", "Park",
  "Kowalski", "Zaman", "Reyes", "Okafor", "Haddad", "Nilsson", "Novak", "Mensah", "Berg",
  "Álvarez", "Sato", "Khalil",
];
const EMAIL_DOMAINS = ["example.com", "example.org", "example.net", "mail.example.com"];
const COUNTRIES = [
  ["United States", 30], ["Canada", 8], ["United Kingdom", 9], ["Germany", 9], ["France", 6],
  ["Netherlands", 4], ["Sweden", 3], ["Japan", 5], ["Australia", 5], ["Brazil", 3], ["Mexico", 3],
  ["Spain", 3], ["Ireland", 2], ["Türkiye", 2], ["Côte d'Ivoire", 1], ["Poland", 2], ["South Korea", 2],
];

function emailFor(first, last) {
  const local = `${slugify(first)}.${slugify(last).split("-")[0]}`;
  const tagged = chance(0.1) ? `${local}+harbor` : local;
  return `${tagged}@${pick(EMAIL_DOMAINS)}`;
}

function orderStatus(ageDays) {
  if (chance(0.03)) return "cancelled";
  if (ageDays < 3) return weighted([["pending", 3], ["paid", 2]]);
  if (ageDays < 8) return weighted([["pending", 1], ["paid", 3], ["shipped", 3]]);
  if (ageDays < 16) return weighted([["paid", 1], ["shipped", 5], ["delivered", 4]]);
  return weighted([["delivered", 23], ["refunded", 1.5], ["shipped", 0.3]]);
}

const ORDER_START_SECONDS = dayOf("2026-05-01") * 86_400;
const ORDER_END_SECONDS = (DATASET_END + 1) * 86_400 - 3600;
const orders = [];
{
  const gaps = Array.from({ length: 500 }, () => rand() + 0.05);
  const total = gaps.reduce((sum, g) => sum + g, 0);
  const span = ORDER_END_SECONDS - ORDER_START_SECONDS;
  let at = ORDER_START_SECONDS;
  for (let i = 0; i < 500; i++) {
    at += Math.max(60, Math.floor((gaps[i] / total) * span));
    const id = i + 1;
    const first = pick(FIRST_NAMES);
    const last = pick(LAST_NAMES);
    const items = weighted([[1, 30], [2, 25], [3, 18], [4, 10], [5, 7], [6, 5], [7, 3], [8, 2]]);
    let subtotal = 0;
    for (let n = 0; n < items; n++) subtotal += pick(products).price_cents;
    const shipping = subtotal >= 7500 ? 0 : 795;
    orders.push({
      id,
      number: `HM-${10000 + id}`,
      customer: `${first} ${last}`,
      email: emailFor(first, last),
      placed_at: isoDateTime(at),
      status: orderStatus((ORDER_END_SECONDS - at) / 86_400),
      items,
      total_cents: subtotal + shipping,
      country: weighted(COUNTRIES),
    });
  }
}

// Business customers whose names need escaping.
Object.assign(orders[41], { customer: "Brown & Daughters <Wholesale>", email: "orders@brown-daughters.example.com" });
Object.assign(orders[287], { customer: "Café \"Le Phare\"", email: "bonjour@lephare.example.net" });

// ---------------------------------------------------------------------------------------------
// Output

function write(name, value) {
  writeFileSync(join(OUT_DIR, name), `${JSON.stringify(value, null, 2)}\n`);
}

write("categories.json", categories);
write("products.json", products);
write("orders.json", orders);
