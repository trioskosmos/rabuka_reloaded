import argparse
import json
from pathlib import Path


def player_features(observations, player):
    return observations[player]["me"]


def action_actor(observation):
    if observation["pending_choice"] and observation["pending_choice_player"] is not None:
        return observation["pending_choice_player"]
    return observation["active_player"]


def action_label(action):
    if action is None:
        return "-"
    params = action.get("parameters") or {}
    card = params.get("card_no") or params.get("card_id") or "-"
    area = params.get("stage_area") or params.get("stage_area_index") or "-"
    cost = params.get("final_cost")
    cost_text = "-" if cost is None else str(cost)
    return f"{action.get('action_type', '?')} card={card} area={area} cost={cost_text}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    records = [json.loads(line) for line in args.trace.read_text(encoding="utf-8").splitlines()]
    game = records[1]
    previous = game["initial"]
    action_number = 0
    lines = [f"trace={args.trace} game={game['game']}"]
    for step_number, step in enumerate(game["steps"]):
        if step["operation"] != "action":
            previous = step["state"]
            continue
        action_number += 1
        before = previous
        after = step["state"]
        actor = action_actor(before["observations"][0])
        before_observation = before["observations"][actor]
        after_observation = after["observations"][actor]
        before_player = player_features(before["observations"], actor)
        after_player = player_features(after["observations"], actor)
        before_features = before_player["features"]
        after_features = after_player["features"]
        selected = step.get("selected")
        lines.append(
            f"{action_number:03d} step={step_number:03d} actor=p{actor + 1} "
            f"phase={before['phase']} turn={before['turn']} "
            f"action={action_label(selected)}"
        )
        lines.append(
            "    before="
            f"cost={before_features['stage_cost']} blades={before_features['active_blades']}/{before_features['total_blades']} "
            f"hearts={before_features['stage_hearts']} live_score={before_features['live_score']} "
            f"success={before_features['success_count']} hand={before_player['hand_size']} "
            f"energy={before_player['active_energy_count']} deck={before_player['main_deck_size']} "
            f"wait={len(before_player['waitroom'])} pending={before_observation['pending_choice']}"
        )
        lines.append(
            "    after ="
            f"cost={after_features['stage_cost']} blades={after_features['active_blades']}/{after_features['total_blades']} "
            f"hearts={after_features['stage_hearts']} live_score={after_features['live_score']} "
            f"success={after_features['success_count']} hand={after_player['hand_size']} "
            f"energy={after_player['active_energy_count']} deck={after_player['main_deck_size']} "
            f"wait={len(after_player['waitroom'])} pending={after_observation['pending_choice']}"
        )
        lines.append(
            "    choice="
            f"{before_observation.get('pending_choice_kind') or '-'} "
            f"options={before_observation.get('pending_choice_option_count', 0)} "
            f"allow_skip={before_observation.get('pending_choice_allow_skip', False)}"
        )
        previous = after
    lines.append(f"actions={action_number}")
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {args.output}")


if __name__ == "__main__":
    main()
