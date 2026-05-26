#!/usr/bin/env python3
import json
import os
import shlex
import subprocess
import textwrap
import time
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path('/workspace/warp')
ART = ROOT / 'validation-artifacts/warpctrl-v2/53d0313d/metadata-config-mutations'
SCREEN = ART / 'screenshots'
LOGS = ART / 'logs'
WARPCTRL = '/workspace/warp/target/debug/warpctrl'
DISPLAY = ':99'
SHA = '53d0313df1f712cf98b1c53e9272c588141da350'
PERMISSION_STATE = 'outside_warp_control=true; metadata_reads=true; metadata_configuration_mutations=true; underlying/app/drive mutation grants not enabled'

SCREEN.mkdir(parents=True, exist_ok=True)
LOGS.mkdir(parents=True, exist_ok=True)

try:
    FONT = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf', 15)
    FONT_BOLD = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf', 15)
except Exception:
    FONT = ImageFont.load_default()
    FONT_BOLD = FONT

manifest_cases = []
log_paths = []
screenshot_paths = []


def rel(p: Path) -> str:
    return str(p.relative_to(ROOT))


def sanitize_name(s: str) -> str:
    out = []
    for ch in s.lower():
        if ch.isalnum():
            out.append(ch)
        elif ch in ['.', '-', '_']:
            out.append(ch.replace('.', '_').replace('-', '_'))
        else:
            out.append('_')
    return ''.join(out).strip('_')[:80]


def capture_ui(path: Path):
    env = os.environ.copy()
    env['DISPLAY'] = DISPLAY
    with open(path, 'wb') as f:
        xwd = subprocess.Popen(['xwd', '-root', '-silent'], stdout=subprocess.PIPE, env=env)
        conv = subprocess.run(['convert', 'xwd:-', str(path)], stdin=xwd.stdout, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        if xwd.stdout:
            xwd.stdout.close()
        xwd.wait(timeout=10)
    if conv.returncode != 0:
        raise RuntimeError(conv.stderr.decode('utf-8', 'replace'))
    screenshot_paths.append(rel(path))


def render_terminal_png(command_display: str, stdout: str, stderr: str, exit_code: int, path: Path):
    lines = []
    lines.append('$ WARPCTRL=/workspace/warp/target/debug/warpctrl')
    lines.append(f'$ {command_display}')
    if stdout:
        lines.extend(stdout.rstrip('\n').splitlines())
    if stderr:
        if stdout:
            lines.append('')
        lines.append('[stderr]')
        lines.extend(stderr.rstrip('\n').splitlines())
    lines.append(f'exit_code={exit_code}')
    wrapped = []
    for line in lines:
        if len(line) <= 180:
            wrapped.append(line)
        else:
            wrapped.extend(textwrap.wrap(line, width=180, replace_whitespace=False, drop_whitespace=False) or [''])
    line_h = 19
    margin = 16
    width = 1620
    height = max(160, margin * 2 + line_h * len(wrapped))
    img = Image.new('RGB', (width, height), (14, 16, 20))
    draw = ImageDraw.Draw(img)
    y = margin
    for i, line in enumerate(wrapped):
        color = (232, 236, 241)
        font = FONT
        if line.startswith('$'):
            color = (121, 214, 255)
            font = FONT_BOLD
        elif line.startswith('exit_code='):
            color = (139, 233, 154) if exit_code == 0 else (255, 121, 121)
            font = FONT_BOLD
        elif line == '[stderr]':
            color = (255, 184, 108)
            font = FONT_BOLD
        draw.text((margin, y), line, fill=color, font=font)
        y += line_h
    img.save(path)
    screenshot_paths.append(rel(path))


def run_raw(args, log_name):
    env = os.environ.copy()
    env['WARPCTRL'] = WARPCTRL
    cp = subprocess.run([WARPCTRL] + args, cwd=str(ROOT), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
    log_path = LOGS / log_name
    log_path.write_text('command: $WARPCTRL ' + ' '.join(shlex.quote(a) for a in args) + '\n' +
                        f'exit_code: {cp.returncode}\n\n[stdout]\n{cp.stdout}\n[stderr]\n{cp.stderr}')
    log_paths.append(rel(log_path))
    return cp, log_path


def parse_json_stdout(cp):
    try:
        return json.loads(cp.stdout)
    except Exception:
        return None


def case(ordinal, family, command_name, args, expected, visible=False, status_expect_zero=True, extra_note=None):
    command_display = '$WARPCTRL ' + ' '.join(shlex.quote(a) for a in args)
    base = f'{ordinal:03d}__outside__{family}__{command_name}'
    before_ui = None
    after_ui = None
    if visible:
        before_ui = SCREEN / f'{ordinal:03d}__outside_before__{family}__{command_name}__ui.png'
        capture_ui(before_ui)
    cp, log_path = run_raw(args, f'{ordinal:03d}__outside__{family}__{command_name}.log')
    term_path = SCREEN / f'{base}__terminal.png'
    render_terminal_png(command_display, cp.stdout, cp.stderr, cp.returncode, term_path)
    time.sleep(0.6)
    if visible:
        after_ui = SCREEN / f'{ordinal:03d}__outside_after__{family}__{command_name}__ui.png'
        capture_ui(after_ui)
    actual_json = parse_json_stdout(cp)
    if status_expect_zero:
        status = 'pass' if cp.returncode == 0 else 'fail'
    else:
        status = 'pass' if cp.returncode != 0 else 'fail'
    actual = {
        'stdout_json': actual_json,
        'stdout': None if actual_json is not None else cp.stdout,
        'stderr': cp.stderr,
    }
    if extra_note:
        actual['note'] = extra_note
    manifest_cases.append({
        'ordinal': ordinal,
        'command': command_display,
        'expected_result': expected,
        'actual_result': actual,
        'exit_code': cp.returncode,
        'invocation_context': 'outside_warp',
        'permission_state': PERMISSION_STATE,
        'terminal_screenshot': rel(term_path),
        'ui_screenshot': rel(after_ui) if after_ui else None,
        'ui_before_screenshot': rel(before_ui) if before_ui else None,
        'log_path': rel(log_path),
        'status': status,
        'skip_reason': None,
    })
    return cp, actual_json


def main():
    # Derive dynamic values using required commands as the first durable cases.
    cp, theme_list = case(1, 'theme', 'theme_list', ['--output-format', 'json', 'theme', 'list'], 'Returns the available theme list as JSON.', visible=False)
    if not isinstance(theme_list, dict):
        theme_list = {'themes': []}
    themes = [t.get('name') for t in theme_list.get('themes', []) if isinstance(t, dict) and t.get('name')]
    current_from_list = next((t.get('name') for t in theme_list.get('themes', []) if isinstance(t, dict) and t.get('is_current')), None)

    cp, theme_get = case(2, 'theme', 'theme_get', ['--output-format', 'json', 'theme', 'get'], 'Returns current theme state as JSON.', visible=False)
    original_theme = (theme_get or {}).get('name') or current_from_list or (themes[0] if themes else 'Dark')
    original_system = bool((theme_get or {}).get('follow_system_theme', False))
    original_light = (theme_get or {}).get('light_theme') or ('Light' if 'Light' in themes else (themes[0] if themes else original_theme))
    original_dark = (theme_get or {}).get('dark_theme') or ('Dark' if 'Dark' in themes else original_theme)
    set_theme = next((t for t in themes if t != original_theme), original_theme)
    light_theme = 'Light' if 'Light' in themes else next((t for t in themes if 'light' in t.lower()), original_light)
    dark_theme = 'Dark' if 'Dark' in themes else next((t for t in themes if 'dark' in t.lower()), original_dark)

    case(3, 'theme', 'theme_set', ['--output-format', 'json', 'theme', 'set', set_theme], f'Sets active theme to derived theme "{set_theme}".', visible=True)
    case(4, 'theme', 'theme_system_set_true', ['--output-format', 'json', 'theme', 'system-set', 'true'], 'Enables follow-system theme setting.', visible=True)
    case(5, 'theme', 'theme_system_set_false', ['--output-format', 'json', 'theme', 'system-set', 'false'], 'Disables follow-system theme setting.', visible=True)
    case(6, 'theme', 'theme_light_set', ['--output-format', 'json', 'theme', 'light-set', light_theme], f'Sets light system theme to derived theme "{light_theme}".', visible=True)
    case(7, 'theme', 'theme_dark_set', ['--output-format', 'json', 'theme', 'dark-set', dark_theme], f'Sets dark system theme to derived theme "{dark_theme}".', visible=True)

    cp, appearance = case(8, 'appearance', 'appearance_get', ['--output-format', 'json', 'appearance', 'get'], 'Returns current appearance state as JSON.', visible=False)
    original_zoom = (appearance or {}).get('ui_zoom_percent', 100)
    case(9, 'appearance', 'appearance_font_size_increase', ['--output-format', 'json', 'appearance', 'font-size-increase'], 'Increases terminal font size.', visible=True)
    case(10, 'appearance', 'appearance_font_size_decrease', ['--output-format', 'json', 'appearance', 'font-size-decrease'], 'Decreases terminal font size.', visible=True)
    case(11, 'appearance', 'appearance_font_size_reset', ['--output-format', 'json', 'appearance', 'font-size-reset'], 'Resets terminal font size.', visible=True)
    case(12, 'appearance', 'appearance_zoom_increase', ['--output-format', 'json', 'appearance', 'zoom-increase'], 'Increases UI zoom level.', visible=True)
    case(13, 'appearance', 'appearance_zoom_decrease', ['--output-format', 'json', 'appearance', 'zoom-decrease'], 'Decreases UI zoom level.', visible=True)
    case(14, 'appearance', 'appearance_zoom_reset', ['--output-format', 'json', 'appearance', 'zoom-reset'], 'Resets UI zoom level.', visible=True)

    cp, setting_list = case(15, 'setting', 'setting_list', ['--output-format', 'json', 'setting', 'list'], 'Returns allowlisted settings as JSON.', visible=False)
    settings = (setting_list or {}).get('settings', []) if isinstance(setting_list, dict) else []
    by_key = {s.get('key'): s for s in settings if isinstance(s, dict) and s.get('key')}
    safe_setting_key = 'appearance.window.zoom_level' if 'appearance.window.zoom_level' in by_key else next(iter(by_key), 'appearance.window.zoom_level')
    original_setting_value = by_key.get(safe_setting_key, {}).get('value', original_zoom)
    if safe_setting_key == 'appearance.window.zoom_level':
        test_setting_value = 110 if original_setting_value != 110 else 100
    elif by_key.get(safe_setting_key, {}).get('value_type') == 'number':
        test_setting_value = original_setting_value
    else:
        test_setting_value = original_setting_value
    safe_bool_key = next((k for k, v in by_key.items() if v.get('value_type') == 'bool' and k != safe_setting_key), None)
    if not safe_bool_key:
        safe_bool_key = 'terminal.input.syntax_highlighting'
    original_bool_value = by_key.get(safe_bool_key, {}).get('value')

    case(16, 'setting', 'setting_get', ['--output-format', 'json', 'setting', 'get', safe_setting_key], f'Gets safe allowlisted setting "{safe_setting_key}".', visible=False)
    case(17, 'setting', 'setting_set', ['--output-format', 'json', 'setting', 'set', safe_setting_key, json.dumps(test_setting_value)], f'Sets safe allowlisted setting "{safe_setting_key}" to test/original value {test_setting_value!r}.', visible=(safe_setting_key == 'appearance.window.zoom_level'))
    case(18, 'setting', 'setting_toggle', ['--output-format', 'json', 'setting', 'toggle', safe_bool_key], f'Toggles safe boolean setting "{safe_bool_key}".', visible=False)
    case(19, 'keybinding', 'keybinding_list', ['--output-format', 'json', 'keybinding', 'list'], 'Returns keybinding metadata list as JSON.', visible=False)
    case(20, 'keybinding', 'keybinding_get_copy', ['--output-format', 'json', 'keybinding', 'get', 'copy'], 'Returns keybinding metadata for the requested action name "copy".', visible=False)

    # Restore changed settings/theme where possible. These are additional executed invocations and therefore receive screenshots/logs.
    if original_bool_value is not None:
        # Re-read current bool and toggle only if needed.
        cp_tmp, data_tmp = run_raw(['--output-format', 'json', 'setting', 'get', safe_bool_key], 'restore_check_bool.log')
        log_paths.append(rel(LOGS / 'restore_check_bool.log')) if rel(LOGS / 'restore_check_bool.log') not in log_paths else None
        try:
            cur_bool = json.loads(cp_tmp.stdout)['setting']['value']
        except Exception:
            cur_bool = None
        if cur_bool is not None and cur_bool != original_bool_value:
            case(21, 'restore', 'setting_toggle_restore', ['--output-format', 'json', 'setting', 'toggle', safe_bool_key], f'Restores boolean setting "{safe_bool_key}" to original value {original_bool_value}.', visible=False)
    if safe_setting_key == 'appearance.window.zoom_level' and test_setting_value != original_setting_value:
        case(22, 'restore', 'setting_set_restore', ['--output-format', 'json', 'setting', 'set', safe_setting_key, json.dumps(original_setting_value)], f'Restores setting "{safe_setting_key}" to original value {original_setting_value!r}.', visible=True)
    if original_light != light_theme:
        case(23, 'restore', 'theme_light_set_restore', ['--output-format', 'json', 'theme', 'light-set', original_light], f'Restores original light theme "{original_light}".', visible=False)
    if original_dark != dark_theme:
        case(24, 'restore', 'theme_dark_set_restore', ['--output-format', 'json', 'theme', 'dark-set', original_dark], f'Restores original dark theme "{original_dark}".', visible=False)
    if original_system != False:
        case(25, 'restore', 'theme_system_set_restore', ['--output-format', 'json', 'theme', 'system-set', str(original_system).lower()], f'Restores follow-system theme to original value {original_system}.', visible=True)
    if set_theme != original_theme:
        case(26, 'restore', 'theme_set_restore', ['--output-format', 'json', 'theme', 'set', original_theme], f'Restores original active theme "{original_theme}".', visible=True)

    counts = {'pass': 0, 'fail': 0, 'skip': 0}
    for c in manifest_cases:
        counts[c['status']] += 1

    manifest = {
        'agent_name': 'metadata-config-mutations',
        'artifact_branch': 'zach/warpctrl-validation-artifacts/53d0313d/metadata-config-mutations',
        'repository': 'git@github.com:warpdotdev/warp.git',
        'validation_ref': 'origin/zach/warpctrl-validation/53d0313d',
        'exact_sha': SHA,
        'head_sha_verified': subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip() == SHA,
        'head_sha': subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip(),
        'build_commands': [
            'cargo build --manifest-path /workspace/warp/Cargo.toml -p warp --bin warp-oss --bin warpctrl --features warp_control_cli',
            "cargo build --manifest-path /workspace/warp/Cargo.toml -p warp --bin warp-oss --bin warpctrl --features 'warp_control_cli skip_firebase_anonymous_user'",
        ],
        'launch_command': 'DISPLAY=:99 WARPCTRL=/workspace/warp/target/debug/warpctrl /workspace/warp/target/debug/warp-oss',
        'warpctrl_binary': WARPCTRL,
        'notes': [
            'The first build with only warp_control_cli succeeded; the second build added the existing skip_firebase_anonymous_user feature as a validation-only launch aid on the same exact commit to reach a logged-out terminal workspace without credentials.',
            'Outside-Warp local-control permissions were enabled via the Linux private preferences file for the assigned read metadata and metadata/configuration mutation categories.',
            'Computer use advanced the no-credentials onboarding path: Get started -> Just use the terminal -> Get Warping, reaching a logged-out terminal workspace.',
            'keybinding get copy was executed exactly as requested and returned missing_target in this build; it is marked fail rather than skipped.',
        ],
        'derived_values': {
            'theme_set_name': set_theme,
            'light_theme_name': light_theme,
            'dark_theme_name': dark_theme,
            'safe_setting_key': safe_setting_key,
            'safe_boolean_setting_key': safe_bool_key,
            'original_theme': original_theme,
            'original_light_theme': original_light,
            'original_dark_theme': original_dark,
            'original_follow_system_theme': original_system,
            'original_setting_value': original_setting_value,
            'test_setting_value': test_setting_value,
            'original_boolean_setting_value': original_bool_value,
        },
        'counts': counts,
        'cases': manifest_cases,
        'screenshots': sorted(set(screenshot_paths)),
        'logs': sorted(set(log_paths + [rel(LOGS / 'environment.txt'), rel(LOGS / 'warp-oss.log'), rel(LOGS / 'xvfb.log'), rel(LOGS / 'openbox.log')]))
    }
    (ART / 'manifest.json').write_text(json.dumps(manifest, indent=2, sort_keys=False))

    summary_lines = [
        '# Warp Control CLI validation: metadata-config-mutations',
        f'Exact SHA: `{SHA}`',
        f'Artifact branch: `zach/warpctrl-validation-artifacts/53d0313d/metadata-config-mutations`',
        '',
        '## Result counts',
        f'- Pass: {counts["pass"]}',
        f'- Fail: {counts["fail"]}',
        f'- Skip: {counts["skip"]}',
        '',
        '## Notable findings',
        '- Built the requested exact commit with `warp_control_cli` and verified local-control discovery against a running `WarpOss` instance.',
        '- The no-credentials onboarding path reached a logged-out terminal workspace through computer use.',
        '- All assigned theme, appearance, setting, and keybinding commands were executed; `keybinding get copy` returned `missing_target` and is the only failed required command.',
        '- Changed theme/setting state was restored where possible using additional captured `warpctrl` invocations.',
        '',
        '## Commands not executed',
        '- None of the requested commands were skipped.',
        '',
        '## Blockers',
        '- No test credentials were available or required. The validation used logged-out-safe outside-Warp control permissions only.',
    ]
    (ART / 'summary.md').write_text('\n'.join(summary_lines) + '\n')
    print(json.dumps({'counts': counts, 'cases': len(manifest_cases), 'manifest': rel(ART / 'manifest.json')}, indent=2))

if __name__ == '__main__':
    main()
