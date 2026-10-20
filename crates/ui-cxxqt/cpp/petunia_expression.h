#pragma once

#include <QString>
#include <cmath>

/// Avaliador determinístico de expressões numéricas para campos tipados
/// (newdocs UI-001 §7: "expressões só com parser determinístico validado").
///
/// Gramática (sem funções, sem variáveis, sem efeitos colaterais):
///   expr    := term (('+' | '-') term)*
///   term    := power (('*' | '/') power)*
///   power   := unary ('^' power)?
///   unary   := ('+' | '-') unary | primary
///   primary := número | 'pi' | '(' expr ')'
/// Números aceitam '.' ou ',' como separador decimal (nunca ambos) e expoente.
/// Entrada hostil é limitada: tamanho, profundidade e resultado finito.
namespace PetuniaExpression {

constexpr int kMaxLength = 128;
constexpr int kMaxDepth = 160; // ~32 níveis de parênteses (cada nível consome 5)

enum class Status {
    Ok,
    Empty,
    Syntax,
    DivideByZero,
    NonFinite,
    TooComplex,
};

struct Result {
    Status status{Status::Syntax};
    double value{0.0};
    bool ok() const { return status == Status::Ok; }
};

class Parser {
public:
    explicit Parser(const QString &text) : m_text(text) {}

    Result run() {
        if (m_text.trimmed().isEmpty())
            return {Status::Empty, 0.0};
        if (m_text.size() > kMaxLength)
            return {Status::TooComplex, 0.0};
        const double value = expr(0);
        skipSpace();
        if (m_status == Status::Ok && m_pos != m_text.size())
            m_status = Status::Syntax;
        if (m_status == Status::Ok && !std::isfinite(value))
            m_status = Status::NonFinite;
        return {m_status, m_status == Status::Ok ? value : 0.0};
    }

private:
    void fail(Status status) {
        if (m_status == Status::Ok)
            m_status = status;
    }
    void skipSpace() {
        while (m_pos < m_text.size() && m_text.at(m_pos).isSpace())
            ++m_pos;
    }
    bool eat(QChar c) {
        skipSpace();
        if (m_pos < m_text.size() && m_text.at(m_pos) == c) {
            ++m_pos;
            return true;
        }
        return false;
    }

    double expr(int depth) {
        if (depth > kMaxDepth) {
            fail(Status::TooComplex);
            return 0.0;
        }
        double left = term(depth + 1);
        while (m_status == Status::Ok) {
            if (eat(QLatin1Char('+')))
                left += term(depth + 1);
            else if (eat(QLatin1Char('-')))
                left -= term(depth + 1);
            else
                break;
        }
        return left;
    }

    double term(int depth) {
        if (depth > kMaxDepth) {
            fail(Status::TooComplex);
            return 0.0;
        }
        double left = power(depth + 1);
        while (m_status == Status::Ok) {
            if (eat(QLatin1Char('*'))) {
                left *= power(depth + 1);
            } else if (eat(QLatin1Char('/'))) {
                const double right = power(depth + 1);
                if (m_status == Status::Ok && right == 0.0) {
                    fail(Status::DivideByZero);
                    return 0.0;
                }
                left /= right;
            } else {
                break;
            }
        }
        return left;
    }

    double power(int depth) {
        if (depth > kMaxDepth) {
            fail(Status::TooComplex);
            return 0.0;
        }
        const double base = unary(depth + 1);
        if (m_status == Status::Ok && eat(QLatin1Char('^'))) {
            const double exponent = power(depth + 1);
            return std::pow(base, exponent);
        }
        return base;
    }

    double unary(int depth) {
        if (depth > kMaxDepth) {
            fail(Status::TooComplex);
            return 0.0;
        }
        if (eat(QLatin1Char('-')))
            return -unary(depth + 1);
        if (eat(QLatin1Char('+')))
            return unary(depth + 1);
        return primary(depth + 1);
    }

    double primary(int depth) {
        skipSpace();
        if (depth > kMaxDepth) {
            fail(Status::TooComplex);
            return 0.0;
        }
        if (eat(QLatin1Char('('))) {
            const double inner = expr(depth + 1);
            if (m_status == Status::Ok && !eat(QLatin1Char(')')))
                fail(Status::Syntax);
            return inner;
        }
        if (m_text.mid(m_pos, 2).compare(QLatin1String("pi"), Qt::CaseInsensitive) == 0) {
            m_pos += 2;
            return M_PI;
        }
        return number();
    }

    double number() {
        const int start = m_pos;
        bool sawDigit = false;
        bool sawDot = false;
        bool sawComma = false;
        while (m_pos < m_text.size()) {
            const QChar c = m_text.at(m_pos);
            if (c.isDigit()) {
                sawDigit = true;
            } else if (c == QLatin1Char('.') && !sawDot && !sawComma) {
                sawDot = true;
            } else if (c == QLatin1Char(',') && !sawDot && !sawComma) {
                sawComma = true;
            } else {
                break;
            }
            ++m_pos;
        }
        if (!sawDigit) {
            fail(Status::Syntax);
            return 0.0;
        }
        // Expoente opcional: e[+-]?dígitos
        if (m_pos < m_text.size() &&
            (m_text.at(m_pos) == QLatin1Char('e') || m_text.at(m_pos) == QLatin1Char('E'))) {
            int probe = m_pos + 1;
            if (probe < m_text.size() &&
                (m_text.at(probe) == QLatin1Char('+') || m_text.at(probe) == QLatin1Char('-')))
                ++probe;
            if (probe < m_text.size() && m_text.at(probe).isDigit()) {
                while (probe < m_text.size() && m_text.at(probe).isDigit())
                    ++probe;
                m_pos = probe;
            }
        }
        QString literal = m_text.mid(start, m_pos - start);
        literal.replace(QLatin1Char(','), QLatin1Char('.'));
        bool ok = false;
        const double value = literal.toDouble(&ok);
        if (!ok) {
            fail(Status::Syntax);
            return 0.0;
        }
        return value;
    }

    QString m_text;
    int m_pos{0};
    Status m_status{Status::Ok};
};

inline Result evaluate(const QString &text) {
    return Parser(text).run();
}

/// Mensagem curta, não bloqueante, para o estado inválido do campo.
inline QString describe(Status status) {
    switch (status) {
    case Status::Ok: return QString();
    case Status::Empty: return QStringLiteral("Digite um valor ou expressão.");
    case Status::Syntax: return QStringLiteral("Expressão inválida.");
    case Status::DivideByZero: return QStringLiteral("Divisão por zero.");
    case Status::NonFinite: return QStringLiteral("O resultado não é um número finito.");
    case Status::TooComplex: return QStringLiteral("Expressão complexa demais.");
    }
    return QString();
}

} // namespace PetuniaExpression
