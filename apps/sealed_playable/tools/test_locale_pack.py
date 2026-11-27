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
        self.assertEqual(len(self.base),948)
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
        self.assertEqual({key for key in self.base if key.startswith("living.circle.")},{'living.circle.empty', 'living.circle.joined', 'living.circle.elected', 'living.circle.inquiry', 'living.circle.coordinator', 'living.circle.formed', 'living.circle.help', 'living.circle.affiliated', 'living.circle.pixel.hint', 'living.circle.unaffiliated', 'living.circle.member', 'living.circle.partnership'})
        self.assertEqual({key for key in self.base if key.startswith("living.diplomacy.")},{'living.diplomacy.help', 'living.diplomacy.lapsed', 'living.diplomacy.withdrawn', 'living.diplomacy.empty', 'living.diplomacy.pixel.hint', 'living.diplomacy.reconciled', 'living.diplomacy.stonebank', 'living.diplomacy.disputed', 'living.diplomacy.expired', 'living.diplomacy.returned', 'living.diplomacy.received', 'living.diplomacy.accord', 'living.diplomacy.superseded', 'living.diplomacy.petition', 'living.diplomacy.disclosed', 'living.diplomacy.hall', 'living.diplomacy.qinghe', 'living.diplomacy.confirmed'})
        self.assertEqual({key for key in self.base if key.startswith("living.governance.")},{'living.governance.review', 'living.governance.help', 'living.governance.negotiations', 'living.governance.no_award', 'living.governance.account', 'living.governance.pixel.hint', 'living.governance.empty', 'living.governance.routine', 'living.governance.negotiation.last', 'living.governance.audit.delivered', 'living.governance.hall', 'living.governance.shrine', 'living.governance.reserve', 'living.governance.audit.last', 'living.governance.caution', 'living.governance.contributions', 'living.governance.audit.unfulfilled', 'living.governance.board', 'living.governance.restored', 'living.governance.decision'})
        self.assertEqual({key for key in self.base if key.startswith("living.field.")},{'living.field.view.help', 'living.field.view.news', 'living.field.view.meridians', 'living.field.route.arrived', 'living.field.pixel.pages', 'living.field.atlas.count', 'living.field.view.diplomacy', 'living.field.action.brew', 'living.field.site.hall', 'living.field.pixel.last', 'living.field.action.invite', 'living.field.view.factions', 'living.field.action.atlas', 'living.field.action.cache.take', 'living.field.route.none', 'living.field.menu.ascii', 'living.field.view.obligations', 'living.field.site.garden', 'living.field.action.favor.accept', 'living.field.action.talk', 'living.field.action.quest.route', 'living.field.site.shrine', 'living.field.view.profiles', 'living.field.route.unavailable', 'living.field.menu.selection', 'living.field.view.favors', 'living.field.action.quest.accept', 'living.field.site.clinic', 'living.field.pixel.position', 'living.field.action.renounce', 'living.field.pixel.control', 'living.field.action.aidlodge', 'living.field.action.herbalist', 'living.field.action.quest.abandon', 'living.field.route.unknown', 'living.field.pixel.select', 'living.field.action.duty', 'living.field.action.drawqi', 'living.field.action.favor.decline', 'living.field.view.questlog', 'living.field.action.remedy', 'living.field.pixel.atlas.control', 'living.field.pixel.route', 'living.field.view.encounters', 'living.field.view.quests', 'living.field.action.survey', 'living.field.direction.east', 'living.field.action.clear', 'living.field.pixel.no_local', 'living.field.view.bonds', 'living.field.view.social', 'living.field.action.ask', 'living.field.action.aid', 'living.field.action.guard', 'living.field.action.mend', 'living.field.view.travel', 'living.field.action.wanderer', 'living.field.view.favorlog', 'living.field.pixel.memory', 'living.field.view.contacts', 'living.field.pixel.reader.control', 'living.field.page.commitments', 'living.field.ascii.help', 'living.field.pixel.empty', 'living.field.view.agency', 'living.field.page.travel', 'living.field.view.history', 'living.field.pixel.here', 'living.field.view.circles', 'living.field.view.sect', 'living.field.action.save', 'living.field.view.martial', 'living.field.page.system', 'living.field.action.pledge', 'living.field.pixel.current', 'living.field.view.governance', 'living.field.action.attack', 'living.field.action.technique', 'living.field.view.rumors', 'living.field.action.center', 'living.field.action.peddler', 'living.field.action.favor.withdraw', 'living.field.action.follow', 'living.field.action.quest.decline', 'living.field.action.cache.entrust', 'living.field.view.audits', 'living.field.action.route', 'living.field.action.favor.fund', 'living.field.pixel.turn', 'living.field.site.market', 'living.field.action.part', 'living.field.site.patch', 'living.field.route.next', 'living.field.page.local', 'living.field.action.repay', 'living.field.action.drill', 'living.field.action.buy', 'living.field.page.training', 'living.field.view.production', 'living.field.view.politics', 'living.field.pixel.menu.control', 'living.field.menu.invalid', 'living.field.action.treat', 'living.field.view.stories', 'living.field.action.wait', 'living.field.page.journal', 'living.field.direction.north', 'living.field.view.messages', 'living.field.view.contracts', 'living.field.action.tell', 'living.field.action.deliver', 'living.field.site.depot', 'living.field.action.waive', 'living.field.action.circulate', 'living.field.action.practice', 'living.field.action.sponsor', 'living.field.action.channel', 'living.field.action.form', 'living.field.view.knowledge', 'living.field.action.load', 'living.field.action.sell', 'living.field.action.accord', 'living.field.action.join', 'living.field.view.aid', 'living.field.pixel.title', 'living.field.action.gather', 'living.field.view.teams', 'living.field.direction.south', 'living.field.direction.west', 'living.field.site.grove', 'living.field.atlas.legend', 'living.field.action.inspect', 'living.field.action.study', 'living.field.view.cultivation', 'living.field.action.allocate', 'living.field.action.pack'})
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
