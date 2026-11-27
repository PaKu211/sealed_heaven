import json
from pathlib import Path
import tempfile
import unittest
import locale_pack as lp

class LocalePackTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalogs=lp.base_catalogs()
        cls.base=cls.catalogs["living"]["en"]

    def test_source_parity(self):
        self.assertEqual(len(self.base),762)
        expected_contacts = {
            "living.contact.care", "living.contact.talk",
            "living.contact.empty", "living.contact.help",
            "living.contact.line", "living.contact.pixel.hint",
        }
        self.assertEqual({key for key in self.base if key.startswith("living.contact.")},expected_contacts)
        self.assertEqual({key for key in self.base if key.startswith("living.network.")},{'living.network.greeting', 'living.network.aid', 'living.network.pixel.hint', 'living.network.conflict', 'living.network.empty', 'living.network.help'})
        self.assertEqual({key for key in self.base if key.startswith("living.team.")},{'living.team.dispatched', 'living.team.delivered', 'living.team.registered', 'living.team.help', 'living.team.pixel.hint', 'living.team.empty', 'living.team.qinghe', 'living.team.willowford'})
        self.assertEqual({key for key in self.base if key.startswith("living.obligation.")},{'living.obligation.expired', 'living.obligation.lost', 'living.obligation.fulfilled', 'living.obligation.empty', 'living.obligation.changed', 'living.obligation.pixel.hint', 'living.obligation.released', 'living.obligation.help'})
        self.assertEqual({key for key in self.base if key.startswith("living.message.")},{'living.message.goal', 'living.message.result.fulfilled', 'living.message.result.released', 'living.message.help', 'living.message.empty', 'living.message.handoff', 'living.message.result.expired', 'living.message.direct', 'living.message.result.changed', 'living.message.relayed', 'living.message.pixel.hint', 'living.message.result.lost'})
        self.assertEqual({key for key in self.base if key.startswith("living.rumor.")},{'living.rumor.contested', 'living.rumor.verified', 'living.rumor.account', 'living.rumor.line', 'living.rumor.adequate', 'living.rumor.empty', 'living.rumor.goal', 'living.rumor.help', 'living.rumor.reported', 'living.rumor.pixel.hint', 'living.rumor.low', 'living.rumor.local'})
        self.assertEqual({key for key in self.base if key.startswith("living.story.")},{'living.story.empty', 'living.story.help', 'living.story.pixel.hint', 'living.story.checked', 'living.story.dispatch', 'living.story.conflict', 'living.story.register', 'living.story.followup', 'living.story.delivery', 'living.story.lapsed', 'living.story.inquiry', 'living.story.witnessed', 'living.story.opportunity'})
        self.assertEqual({key for key in self.base if key.startswith("living.profile.")},{'living.profile.help', 'living.profile.pixel.hint', 'living.profile.empty', 'living.profile.vocation', 'living.profile.line', 'living.profile.inquiry', 'living.profile.partnership'})
        self.assertEqual(self.base.keys(),self.catalogs["living"]["id"].keys())
        self.assertEqual(len(self.catalogs["ui"]["en"]),203)

    def test_escaped_and_reordered_named_parameters(self):
        self.assertEqual(lp.parameters("{actor} {{literal}} {count}"),{"actor","count"})
        self.assertEqual(lp.parameters("{count} {actor}"),{"actor","count"})
        for bad in ("{","}","{Bad}","{9}","{a-b}","{a b}"):
            with self.subTest(bad=bad),self.assertRaises(ValueError):
                lp.parameters(bad)

    def test_reject_missing_unknown_parameter_and_key(self):
        for override in [
            {"living.save.error":"no parameter"},
            {"living.save.ok":"{extra}"},
            {"missing.key":"x"},
            {"living.save.ok":""},
            {"living.save.ok":"\x1b[2J"},
            {"living.save.ok":123},
        ]:
            with self.subTest(override=override),self.assertRaises(ValueError):
                lp.validate("living",override,self.base)

    def test_partial_translation_reports_fallback(self):
        self.assertEqual(lp.validate("living",{"living.save.ok":"Tersimpan"},self.base),(1,len(self.base)-1))

    def test_duplicate_decoded_keys_and_invalid_utf8(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/"pack.json"
            for raw in [b'{"key":"a","k\\u0065y":"b"}',b'\xff',b' '* (lp.MAX_PACK+1)]:
                p.write_bytes(raw)
                with self.assertRaises((ValueError,UnicodeError)):
                    lp.read_json(p)

    def test_export_and_validate_all_files(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/"export"
            lp.export_pack(p,self.catalogs)
            for kind in lp.KINDS:
                for lang in lp.LANGS:
                    self.assertEqual(lp.validate(kind,lp.read_json(p/f"{kind}_{lang}.json"),self.catalogs[kind]["en"],complete=True)[1],0)
            self.assertEqual(json.loads((p/"living_context.json").read_text(encoding="utf-8")).keys(),self.base.keys())
            with self.assertRaises(FileExistsError):
                lp.export_pack(p,self.catalogs)

    def test_overwrite_requires_explicit_replace(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/"living_id.json"
            lp.write_new(p,b'{}')
            with self.assertRaises(FileExistsError):
                lp.write_new(p,b'{"x":"y"}')
            self.assertEqual(p.read_bytes(),b'{}')
            lp.write_new(p,b'{"x":"y"}',replace=True)
            self.assertEqual(p.read_bytes(),b'{"x":"y"}')

if __name__=="__main__":
    unittest.main()
