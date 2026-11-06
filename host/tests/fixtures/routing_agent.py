"""Deterministic ACP fixture for routing ordering/resume/acknowledgement tests. No model or network."""
import json
import os
import sys
import uuid

model, effort, fast = 'gpt-6.1-sol', 'medium', 'on'
sid = None

def send(value):
    print(json.dumps({'jsonrpc': '2.0', **value}), flush=True)

def options():
    def option(key, current, values, category):
        return {'id': key, 'type': 'select', 'category': category, 'currentValue': current,
                'options': [{'value': value, 'name': value} for value in values]}
    return {'configOptions': [
        option('model', model, ['gpt-6.1-sol', 'gpt-6.1-terra'], 'model'),
        option('reasoning_effort', effort, ['low', 'medium', 'high'], 'thought_level'),
        option('fast-mode', fast, ['on', 'off'], 'model_config')]}

for line in sys.stdin:
    message = json.loads(line)
    method, rid = message.get('method'), message.get('id')
    if rid is None or method is None:
        continue
    p = message.get('params', {})
    with open('calls.jsonl', 'a', encoding='utf8') as f:
        f.write(json.dumps({'method': method, 'configId': p.get('configId'), 'value': p.get('value'),
                            'model': model, 'effort': effort, 'fast': fast, 'sid': sid}) + '\n')
    if method == 'initialize':
        result = {'protocolVersion': 1, 'agentInfo': {'name': 'routing-fixture'},
                  'agentCapabilities': {'loadSession': True}}
    elif method in ['session/new', 'session/load']:
        sid = p.get('sessionId') or str(uuid.uuid4())
        result = {'sessionId': sid, **({} if method == 'session/load' and os.path.exists('empty-load') else options())}
    elif method == 'session/set_config_option':
        key, value = p['configId'], p['value']
        if key == 'model': model = value
        if key == 'reasoning_effort' and not os.path.exists('ignore-effort'): effort = value
        if key == 'fast-mode': fast = value
        result = options()
    elif method == 'session/prompt':
        send({'method': 'session/update', 'params': {'sessionId': sid, 'update': {
            'sessionUpdate': 'agent_message_chunk', 'content': {'type': 'text', 'text': f'{model} {effort} {fast}'}}}})
        result = {'stopReason': 'end_turn'}
    else:
        result = {}
    send({'id': rid, 'result': result})
