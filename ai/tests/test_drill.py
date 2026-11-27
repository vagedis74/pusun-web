import json
import sys
import pathlib
import pytest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import drill  # noqa: E402


def ball(point, stroke="forehand", player=1, **kw):
    return {"point": point, "height": kw.get("height", 2000), "speed": kw.get("speed", 130),
            "interval": kw.get("interval", 30), "spin": kw.get("spin", 2),
            "player": player, "stroke": stroke, "target": "deep cross", "cue": "early"}


def singles_fh():
    return {"name": "Inferno", "summary": "s", "why_hard": "w", "pause": "none",
            "players": [{"label": "Player 1", "start": "centre"}],
            "balls": [ball(p) for p in (28, 21, 24, 27, 26, 20, 14, 25)],
            "structure": {"sets": 4, "loops_per_set": 3, "rest_seconds": 60},
            "coaching_points": ["a", "b"], "safety": "x"}


def doubles_both():
    balls = []
    for i, p in enumerate((22, 28, 16, 20, 23, 27, 9, 13)):
        balls.append(ball(p, "backhand" if p % 7 in (1, 2, 3) else "forehand", player=1 + i % 2))
    return {"name": "Storm", "pause": "short", "balls": balls,
            "players": [{"label": "A", "start": "baseline"}, {"label": "B", "start": "net"}]}


def test_singles_needs_exactly_8():
    raw = singles_fh()
    assert len(drill.validate(raw, "singles", "forehand").balls) == 8
    raw["balls"] = raw["balls"][:7]
    with pytest.raises(drill.DrillError, match="exactly 8"):
        drill.validate(raw, "singles", "forehand")


def test_doubles_needs_4_each():
    d = drill.validate(doubles_both(), "doubles", "both")
    assert [sum(b.player == p for b in d.balls) for p in (1, 2)] == [4, 4]
    raw = doubles_both()
    raw["balls"][1]["player"] = 1  # now 5 and 3
    with pytest.raises(drill.DrillError, match="4 balls for each player"):
        drill.validate(raw, "doubles", "both")


def test_side_is_enforced():
    raw = singles_fh()
    raw["balls"][3]["stroke"] = "backhand"
    with pytest.raises(drill.DrillError, match="non-forehand"):
        drill.validate(raw, "singles", "forehand")
    with pytest.raises(drill.DrillError, match="non-backhand"):
        drill.validate(singles_fh(), "singles", "backhand")


def test_values_are_snapped_and_clamped():
    raw = singles_fh()
    raw["balls"][0].update(height=9999, speed=60, interval=12, spin=-9)
    b = drill.validate(raw, "singles", "forehand").balls[0]
    assert (b.height, b.speed, b.interval, b.spin) == (4500, drill.SPEED_FLOOR, 25, -6)


def test_reused_point_plays_like_first():
    raw = singles_fh()
    raw["balls"][7] = ball(28, speed=110)
    d = drill.validate(raw, "singles", "forehand")
    assert d.balls[7].speed == d.balls[0].speed == 130
    assert d.notes


def test_route_line_matches_app_format():
    d = drill.validate(singles_fh(), "singles", "forehand")
    name, points, pause, params = d.route_line().split("\t")
    assert name == "AI: Inferno" and pause == "0"
    assert points.split(",")[0] == "28"
    first = params.split(";")[0].split(",")
    # lr of column 7, ud, speed, interval, spin (50 + 5*2), id
    assert first == ["270", "2000", "130", "30", "60", "28"]
    assert len(params.split(";")) == len(set(points.split(",")))


def test_extract_json_tolerates_fences():
    txt = "Here you go:\n```json\n" + json.dumps(singles_fh()) + "\n```"
    assert drill.extract_json(txt)["name"] == "Inferno"
    with pytest.raises(drill.DrillError):
        drill.extract_json("no json here")
