import json
import os

with open('extracted/project.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

assets_map = []
for target in data['targets']:
    target_name = target['name']
    for costume in target.get('costumes', []):
        assets_map.append({
            'target': target_name,
            'type': 'costume',
            'name': costume['name'],
            'md5': costume['md5ext']
        })
    for sound in target.get('sounds', []):
        assets_map.append({
            'target': target_name,
            'type': 'sound',
            'name': sound['name'],
            'md5': sound['md5ext']
        })

with open('assets_mapping.json', 'w', encoding='utf-8') as f:
    json.dump(assets_map, f, indent=4, ensure_ascii=False)

print(f"Mapped {len(assets_map)} assets to assets_mapping.json")
