-- lantern: a paper lantern swaying on its cord over still water, with a soft glow
-- around it and sparks drifting up into the night.
--
-- An example scene for Zacess Scenes. Copy it into your scenes folder to run it:
--
--   macOS and Linux:  ~/.config/zacess/scenes/lantern.lua
--   Windows:          %APPDATA%\zacess\scenes\lantern.lua
--
-- then run `zacess-scenes lantern` or `zacess-scenes watch --scene lantern`.
-- The API is described in docs/lua.md.

-- seconds for one full sway, left and back again
local SWAY_PERIOD = 7
-- seconds for one slow breath of the glow; anything that changes in place stays slow
local GLOW_PERIOD = 5

-- the two lantern sizes, drawn from their top left corner
local LARGE = {
  "   _|_   ",
  "  /=-=\\  ",
  " ( ### ) ",
  " ( ### ) ",
  " ( ### ) ",
  "  \\=-=/  ",
  "    |    ",
  "    '    ",
}
local SMALL = {
  " _|_ ",
  "(###)",
  " \\_/ ",
  "  '  ",
}

local stars = {}
local sparks = {}
local spawn = 0

local function place_stars(ctx)
  stars = {}
  local count = math.max(4, math.floor(ctx.width * ctx.height / 90 * ctx.busy))
  for i = 1, count do
    stars[i] = {
      x = ctx.random(0, ctx.width - 1),
      y = ctx.random(0, math.max(0, ctx.height - 4)),
      dot = ctx.random() < 0.7,
    }
  end
end

-- rows of water along the bottom: more on a tall screen
local function water_rows(ctx)
  if ctx.height >= 24 then
    return 4
  elseif ctx.height >= 14 then
    return 2
  end
  return 1
end

-- which body to draw and where its top row sits: hung about a quarter of the way down, with
-- room left for the water below
local function body(ctx)
  if ctx.height >= 16 and ctx.width >= 24 then
    local room = ctx.height - water_rows(ctx) - #LARGE - 2
    return LARGE, math.max(2, math.min(math.floor(ctx.height / 4), room))
  end
  return SMALL, 1
end

-- the role of each character of the lantern, so the frame, the paper and the light differ
local function role_of(ch)
  if ch == "#" then
    return "bright_yellow"
  elseif ch == "(" or ch == ")" then
    return "red"
  elseif ch == "|" or ch == "'" then
    return "grey"
  end
  return "yellow"
end

return {
  name = "lantern",
  kind = "pass",
  summary = "A paper lantern swaying over still water, with a soft glow and drifting sparks",
  captions = {
    "A warm light over a clean build.",
    "Everything compiles. The lantern stays lit.",
    "Quiet water, steady light, no errors.",
    "All good. Sit with it a moment.",
  },

  init = function(ctx)
    place_stars(ctx)
  end,

  resize = function(ctx)
    place_stars(ctx)
    sparks = {}
  end,

  frame = function(ctx)
    local w, h = ctx.width, ctx.height
    local lantern, top = body(ctx)
    local lw = #lantern[1]
    local rows = water_rows(ctx)
    local water = h - rows

    -- the sway moves the lantern a few cells either side of the centre; ctx.pace slows it
    -- under reduced motion and never speeds it past the standard rate
    local reach = math.max(1, math.floor(w / 16))
    local swing = math.sin(ctx.time * 2 * math.pi / SWAY_PERIOD * ctx.pace)
    local cx = math.floor(w / 2 + swing * reach + 0.5)
    local left = cx - math.floor(lw / 2)

    -- stars behind everything
    for _, star in ipairs(stars) do
      put(star.x, star.y, star.dot and "." or "+", "muted")
    end

    -- the cord from the top of the screen down to the lantern
    for y = 0, top - 1 do
      put(cx, y, "|", "grey")
    end

    -- a ring of glow that breathes between two sizes, one step every few seconds
    local breath = math.sin(ctx.time * 2 * math.pi / GLOW_PERIOD * ctx.pace)
    local glow = breath > 0 and 2 or 1
    local middle = top + math.floor(#lantern / 2)
    for dy = -glow, glow do
      local y = middle + dy
      put(left - 1 - glow + math.abs(dy), y, ".", "yellow")
      put(left + lw + glow - math.abs(dy), y, ".", "yellow")
    end

    -- the lantern itself, cell by cell so each part takes its own colour
    for row, line in ipairs(lantern) do
      for i = 1, #line do
        local ch = line:sub(i, i)
        if ch ~= " " then
          put(left + i - 1, top + row - 1, ch, role_of(ch))
        end
      end
    end

    -- sparks rise from the top of the lantern, drift with the sway and fade as they climb
    if not ctx.reduced or #sparks < 2 then
      spawn = spawn + ctx.dt * 1.2 * ctx.busy
    end
    while spawn >= 1 do
      spawn = spawn - 1
      sparks[#sparks + 1] = {
        -- from the edges of the lid, clear of the cord
        x = cx + (ctx.random() < 0.5 and -2 or 2),
        y = top + 1,
        rise = (0.8 + ctx.random() * 0.8) * ctx.speed,
        drift = (ctx.random() - 0.5) * 0.6 * ctx.speed,
      }
    end
    local kept = {}
    for _, spark in ipairs(sparks) do
      spark.y = spark.y - spark.rise * ctx.dt
      spark.x = spark.x + spark.drift * ctx.dt
      if spark.y >= 0 then
        local climbed = top + 1 - spark.y
        local ch, role = "*", "bright_yellow"
        if climbed > 3 then
          ch, role = ".", "muted"
        elseif climbed > 1.5 then
          ch, role = "+", "yellow"
        end
        put(math.floor(spark.x + 0.5), math.floor(spark.y), ch, role)
        kept[#kept + 1] = spark
      end
    end
    sparks = kept

    -- still water along the bottom: gentle ripples drifting slowly sideways, with the
    -- lantern's light laid across it in a narrowing column
    if water > top + #lantern then
      local drift = math.floor(ctx.time * 0.6 * ctx.speed)
      for r = 0, rows - 1 do
        local y = water + r
        local line = {}
        for x = 0, w - 1 do
          local k = (x + drift * (r % 2 == 0 and 1 or -1) + r * 3) % 9
          line[#line + 1] = (k == 0 or k == 5) and "-" or "~"
        end
        put(0, y, table.concat(line), r == 0 and "cyan" or "blue")
        local shine = math.max(1, math.floor(lw / 2) - r)
        put(cx - math.floor(shine / 2), y, string.rep(r == 0 and "=" or "-", shine), "yellow")
      end
    end
  end,

  on_recovered = function(ctx)
    return "Fixed. The lantern is lit again."
  end,
}
