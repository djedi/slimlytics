import importlib.util
import pathlib
import unittest
from types import SimpleNamespace
from unittest.mock import patch


SCRIPT_PATH = pathlib.Path(__file__).with_name('local_pr_review.py')
SPEC = importlib.util.spec_from_file_location('local_pr_review', SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f'Unable to load {SCRIPT_PATH}')
local_pr_review = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(local_pr_review)


class DiffParsingTest(unittest.TestCase):
    def test_rust_symbols_find_related_callers(self):
        diff = "\n".join([
            "@@ -1 +1 @@ async fn collect_events() {",
            "+pub struct SiteConfig {",
            "+enum TrafficClass {",
            "+trait EventSink {",
            "+impl EventSink for Collector {",
        ])
        self.assertEqual(local_pr_review.changed_symbols(diff),
                         ['collect_events', 'SiteConfig', 'TrafficClass', 'EventSink'])

    def test_added_lines_tracks_only_reviewable_right_side_lines(self):
        diff = """diff --git a/app.py b/app.py
--- a/app.py
+++ b/app.py
@@ -10,3 +10,4 @@
 unchanged
-old
+new
+second
 unchanged
"""

        self.assertEqual(
            local_pr_review.added_lines_by_path(diff),
            {'app.py': {11, 12}},
        )


class FindingValidationTest(unittest.TestCase):
    def test_validate_findings_drops_invalid_and_duplicate_comments(self):
        review = {
            'summary': 'Found one issue.',
            'findings': [
                {
                    'path': 'app.py',
                    'line': 11,
                    'severity': 'high',
                    'title': 'Unsafe fallback',
                    'body': 'This fallback bypasses validation.',
                },
                {
                    'path': 'app.py',
                    'line': 11,
                    'severity': 'high',
                    'title': 'Unsafe fallback',
                    'body': 'This fallback bypasses validation.',
                },
                {
                    'path': 'app.py',
                    'line': 10,
                    'severity': 'low',
                    'title': 'Not on added line',
                    'body': 'Cannot be posted inline.',
                },
                {
                    'path': '../secret',
                    'line': 1,
                    'severity': 'critical',
                    'title': 'Invalid path',
                    'body': 'No.',
                },
            ],
        }

        validated = local_pr_review.validate_review(review, {'app.py': {11}})

        self.assertEqual(validated['summary'], 'Found one issue.')
        self.assertEqual(len(validated['findings']), 1)
        self.assertEqual(validated['findings'][0]['path'], 'app.py')
        self.assertEqual(validated['findings'][0]['line'], 11)

    def test_validate_review_rejects_non_list_findings(self):
        with self.assertRaisesRegex(ValueError, 'findings must be a JSON array'):
            local_pr_review.validate_review(
                {'summary': 'Malformed.', 'findings': None},
                {'app.py': {11}},
            )


class ReviewerBoundaryTest(unittest.TestCase):
    def test_zero_toolset_preflight_rejects_any_exposed_tool(self):
        exposed = SimpleNamespace(stdout='["web_search"]', returncode=0)
        with patch.object(local_pr_review.subprocess, 'run', return_value=exposed):
            with self.assertRaisesRegex(RuntimeError, 'exposes tools'):
                local_pr_review.assert_zero_toolset()

    def test_current_head_check_rejects_stale_review(self):
        with patch.object(
            local_pr_review,
            'gh_api',
            return_value={'head': {'sha': 'new'}},
        ):
            with self.assertRaisesRegex(RuntimeError, 'head changed before posting'):
                local_pr_review.assert_current_head('example/repo', 1, 'expected')

    def test_model_output_requires_one_trailing_json_object(self):
        parsed = local_pr_review.parse_model_json(
            'session_id: test\n{"summary":"ok","findings":[]}\n'
        )
        self.assertEqual(parsed['summary'], 'ok')
        parsed_with_warning = local_pr_review.parse_model_json(
            '  ⚠ tirith security scanner enabled but not available '
            '— command scanning will use pattern matching only\r\n\n'
            'session_id: test\n{"summary":"ok","findings":[]}\n'
        )
        self.assertEqual(parsed_with_warning['summary'], 'ok')
        with self.assertRaises(ValueError):
            local_pr_review.parse_model_json('not json')
        with self.assertRaises(ValueError):
            local_pr_review.parse_model_json(
                '{"summary":"ignored","findings":[]} '
                '{"summary":"accepted","findings":[]}'
            )
        with self.assertRaises(ValueError):
            local_pr_review.parse_model_json(
                'unexpected prose\n{"summary":"accepted","findings":[]}'
            )

    def test_reviewer_uses_requested_model(self):
        completed = SimpleNamespace(stdout='{"summary":"ok","findings":[]}')
        with (
            patch.object(local_pr_review, 'assert_zero_toolset'),
            patch.object(local_pr_review.pathlib.Path, 'read_text', return_value='{}'),
            patch.object(local_pr_review.subprocess, 'run', return_value=completed) as run,
        ):
            local_pr_review.run_model_review('', local_pr_review.pathlib.Path('schema.json'))
        command = run.call_args.args[0]
        self.assertEqual(command[command.index('--model') + 1], 'gpt-6.1-sol')

    def test_github_pagination_reads_every_page(self):
        first_page = [{'id': index} for index in range(100)]
        with patch.object(
            local_pr_review,
            'gh_api',
            side_effect=[first_page, [{'id': 100}]],
        ) as api:
            items = local_pr_review.gh_api_pages('repos/example/repo/issues/1/comments')

        self.assertEqual(len(items), 101)
        self.assertIn('page=2', api.call_args_list[1].args[0])

    def test_diff_fetch_aborts_when_head_changes(self):
        completed = SimpleNamespace(stdout='diff --git a/a b/a\n', returncode=0)
        with (
            patch.object(
                local_pr_review,
                'gh_api',
                side_effect=[{'head': {'sha': 'expected'}}, {'head': {'sha': 'new'}}],
            ),
            patch.object(local_pr_review.subprocess, 'run', return_value=completed),
        ):
            with self.assertRaisesRegex(RuntimeError, 'changed during review'):
                local_pr_review.fetch_diff('example/repo', 1, 'expected')


class ContextAndSuggestionTest(unittest.TestCase):
    DIFF = """diff --git a/app.py b/app.py
--- a/app.py
+++ b/app.py
@@ -10,3 +10,4 @@ def merge_transactions(cls, ids):
 unchanged
-old
+new
+second
 unchanged
"""

    def test_context_lines_are_commentable_but_not_added(self):
        added, commentable = local_pr_review.diff_lines_by_path(self.DIFF)
        self.assertEqual(added, {'app.py': {11, 12}})
        self.assertEqual(commentable, {'app.py': {10, 11, 12, 13}})

    def test_changed_symbols_come_from_hunk_headers_and_changed_lines(self):
        diff = self.DIFF + "+def restore(self):\n-class Helper:\n+def save(self):\n+def test_x():\n"
        self.assertEqual(
            local_pr_review.changed_symbols(diff),
            ['merge_transactions', 'restore', 'Helper'],
        )

    def finding(self, **extra):
        return {
            'path': 'app.py', 'line': 12, 'severity': 'medium',
            'title': 'Bug', 'body': 'Explained.', **extra,
        }

    def test_suggestion_kept_only_when_its_whole_range_is_commentable(self):
        allowed = {'app.py': {10, 11, 12, 13}}
        kept = local_pr_review.validate_review(
            {'summary': 's', 'findings': [self.finding(start_line=11, suggestion='a\nb\n')]}, allowed
        )['findings'][0]
        self.assertEqual((kept['start_line'], kept['suggestion']), (11, 'a\nb'))

        dropped = local_pr_review.validate_review(
            {'summary': 's', 'findings': [self.finding(start_line=5, suggestion='x')]}, allowed
        )['findings'][0]
        self.assertNotIn('suggestion', dropped)
        self.assertEqual(dropped['line'], 12)

    def test_comment_renders_suggestion_fence_and_multiline_range(self):
        finding = self.finding(start_line=11, suggestion='use ```code```', pre_existing=True)
        comment = local_pr_review.finding_comment(finding, 'f' * 20)
        self.assertIn('(pre-existing)', comment['body'])
        self.assertIn('````suggestion\nuse ```code```\n````', comment['body'])
        self.assertEqual((comment['start_line'], comment['start_side']), (11, 'RIGHT'))

    def test_file_summaries_only_for_files_in_the_diff(self):
        review = local_pr_review.validate_review(
            {
                'summary': 's',
                'findings': [],
                'file_summaries': [
                    {'path': 'app.py', 'summary': 'Adds a | pipe'},
                    {'path': 'other.py', 'summary': 'Not in diff'},
                ],
            },
            {'app.py': {10}},
        )
        self.assertEqual(review['file_summaries'], [{'path': 'app.py', 'summary': 'Adds a | pipe'}])
        body = local_pr_review.review_body(review, 'abc', 0)
        self.assertIn('| `app.py` | Adds a \\| pipe |', body)

    def test_fetch_file_skips_binary_and_non_utf8_content(self):
        for payload in (b'\x89PNG\x00\x01', b'caf\xe9'):
            completed = SimpleNamespace(stdout=payload, returncode=0)
            with patch.object(local_pr_review.subprocess, 'run', return_value=completed):
                self.assertIsNone(local_pr_review.fetch_file_at('example/repo', 'a', 'sha'))
        completed = SimpleNamespace(stdout='café'.encode(), returncode=0)
        with patch.object(local_pr_review.subprocess, 'run', return_value=completed):
            self.assertEqual(local_pr_review.fetch_file_at('example/repo', 'a', 'sha'), 'café')

    def test_trusted_file_reads_stay_inside_the_checkout(self):
        root = pathlib.Path(__file__).resolve().parent
        self.assertIsNone(local_pr_review.read_trusted_file(root, '../../../etc/passwd'))

    def test_context_respects_budget_and_orders_changed_before_related(self):
        with (
            patch.object(local_pr_review, 'fetch_file_at', return_value='x' * 100),
            patch.object(local_pr_review, 'read_trusted_file', return_value='y' * 100),
            patch.object(local_pr_review, 'related_paths', return_value=['a.py', 'b.py']),
            patch.object(local_pr_review, 'MAX_CONTEXT_BYTES', 600),
        ):
            instructions, blocks = local_pr_review.build_context(
                'example/repo', 'sha', self.DIFF, pathlib.Path('.')
            )
        self.assertIn('review-instructions.md', instructions)
        self.assertIn('version="pr-head"', blocks[0])
        self.assertEqual(len(blocks), 2)  # budget runs out before the second related file

    def test_update_previous_overview_when_no_new_comments(self):
        review = {'summary': 's', 'file_summaries': [], 'findings': []}
        own = {'id': 7, 'body': local_pr_review.SUMMARY_MARKER, 'user': {'login': 'github-actions[bot]'}}
        calls = []

        def fake_pages(endpoint):
            if endpoint.endswith('/reviews'):
                return [own]
            if endpoint.endswith('/issues/1/comments'):
                return [dict(own, id=9)]
            return []

        with (
            patch.object(local_pr_review, 'gh_api_pages', side_effect=fake_pages),
            patch.object(local_pr_review, 'assert_current_head'),
            patch.object(local_pr_review, 'gh_api', side_effect=lambda e, method='GET', payload=None: calls.append((method, e))),
        ):
            local_pr_review.post_review('example/repo', 1, 'sha', review)

        self.assertEqual(
            calls,
            [
                ('PUT', 'repos/example/repo/pulls/1/reviews/7'),
                ('DELETE', 'repos/example/repo/issues/comments/9'),
            ],
        )


if __name__ == '__main__':
    unittest.main()
