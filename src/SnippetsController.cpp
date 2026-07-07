#include "SnippetsController.h"
#include "Portable.h"
#include "CodeEditor.h"
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>

SnippetsController::SnippetsController(QObject* parent) : QObject(parent) { load(); }

QString SnippetsController::configPath() {
    return Portable::dataDir() + QStringLiteral("/alexcode-snippets.json");
}

void SnippetsController::load() {
    const QString cfgPath = configPath();
    if (!QFileInfo::exists(cfgPath)) {
        QFile f(cfgPath);
        if (f.open(QIODevice::WriteOnly)) {
            f.write(QByteArray(
"[\n"
"  {\"trigger\": \"forr\",  \"language\": \"cpp\",    \"body\": \"for (int i = 0; i < ${1:n}; ++i) {\\n    $0\\n}\"},\n"
"  {\"trigger\": \"main\",  \"language\": \"cpp\",    \"body\": \"int main(int argc, char** argv) {\\n    $0\\n    return 0;\\n}\"},\n"
"  {\"trigger\": \"deff\",  \"language\": \"python\", \"body\": \"def ${1:name}():\\n    $0\"},\n"
"  {\"trigger\": \"ifmain\",\"language\": \"python\", \"body\": \"if __name__ == \\\"__main__\\\":\\n    $0\"},\n"
"  {\"trigger\": \"todo\",  \"language\": \"\",       \"body\": \"TODO($0): \"}\n"
"]\n"));
        }
    }
    m_defs.clear();
    QFile f(cfgPath);
    if (!f.open(QIODevice::ReadOnly)) return;
    for (const QJsonValue& v : QJsonDocument::fromJson(f.readAll()).array()) {
        const QJsonObject o = v.toObject();
        SnippetDef d{ o.value("trigger").toString(),
                      o.value("language").toString().toLower(),
                      o.value("body").toString() };
        if (!d.trigger.isEmpty() && !d.body.isEmpty()) m_defs.append(d);
    }
}

void SnippetsController::applyTo(CodeEditor* editor) const {
    if (!editor) return;
    const QString lang = editor->property("language").toString();   // "C/C++" / "Python" / …
    const QString key = lang == "C/C++" ? QStringLiteral("cpp")
                      : lang == "Python" ? QStringLiteral("python") : QString();
    QHash<QString, QString> map;
    for (const SnippetDef& d : m_defs)
        if (d.language.isEmpty() || d.language == key)
            map.insert(d.trigger, d.body);
    editor->setSnippets(map);
}
