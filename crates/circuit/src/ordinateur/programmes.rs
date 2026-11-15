//! Des programmes pour l'ordinateur miniature, en assembleur.

/// Affiche la suite de Fibonacci (un octet par terme) tant qu'elle tient sur 8 bits.
pub const FIBONACCI: &str = "\
; La suite de Fibonacci : 0, 1, 1, 2, 3, 5, 8... jusqu'au dernier terme qui tient sur 8 bits.
a = 0
b = 1
t = 2
        LDI 0
        STA a
        LDI 1
        STA b
boucle: LDA a
        OUT             ; affiche le terme courant
        ADD b
        JC fin          ; la somme dépasse 255 : terminé
        STA t
        LDA b
        STA a
        LDA t
        STA b
        JMP boucle
fin:    HLT
";

/// Lit deux chiffres au clavier et affiche leur produit en décimal : `6*7=42`.
pub const CALCULATRICE: &str = "\
; Tape deux chiffres : la machine affiche leur produit.
x = 0                   ; premier chiffre
n = 1                   ; second chiffre, puis compteur de la multiplication
p = 2                   ; produit, puis unités
d = 3                   ; dizaines
k = 4                   ; touche lue
t1:     IN              ; attend une touche
        JZ t1
        OUT
        SUBI '0'
        STA x
r1:     IN              ; attend qu'elle soit relâchée
        JZ t2
        JMP r1
t2:     IN              ; attend la seconde touche
        JZ t2
        STA k
        LDI '*'
        OUT
        LDA k
        OUT
        SUBI '0'
        STA n
        LDI '='
        OUT
        LDI 0
        STA p
mul:    LDA n           ; p = x + x + ... (n fois)
        JZ affiche
        SUBI 1
        STA n
        LDA p
        ADD x
        STA p
        JMP mul
affiche: LDI 0
        STA d
dix:    LDA p           ; compte les dizaines en retirant 10 tant que c'est possible
        SUBI 10
        JC plus
        LDA d
        JZ unite        ; pas de zéro en tête
        ADDI '0'
        OUT
unite:  LDA p
        ADDI '0'
        OUT
        HLT
plus:   STA p
        LDA d
        ADDI 1
        STA d
        JMP dix
";

/// Recopie à l'écran ce qui est tapé au clavier, les minuscules passées en majuscules.
/// Ne s'arrête jamais.
pub const MAJUSCULES: &str = "\
; Recopie les touches à l'écran, en majuscules.
k = 0
attend: IN
        JZ attend
        STA k
        SUBI 'a'        ; avant 'a' : pas une minuscule
        JC peut_etre
        JMP tel_quel
peut_etre: SUBI 26      ; 'a'..'z' donne 0..25 : la soustraction de 26 emprunte
        JC tel_quel
        LDA k
        SUBI 32
        JMP ecrit
tel_quel: LDA k
ecrit:  OUT
relache: IN             ; attend que la touche soit relâchée
        JZ attend
        JMP relache
";

/// L'ordinateur qui envoie : chaque touche, passée en majuscule, part dans la boîte aux
/// lettres, suivie d'un 0 qui la vide. Au démarrage, un 0 vide la boîte.
pub const ENVOI_MAJUSCULES: &str = "\
; Envoie chaque touche en majuscule, puis un 0.
k = 0
        LDI 0
        OUT             ; la boîte aux lettres part vide
attend: IN
        JZ attend
        STA k
        SUBI 'a'        ; avant 'a' : pas une minuscule
        JC peut_etre
        JMP tel_quel
peut_etre: SUBI 26      ; 'a'..'z' donne 0..25 : la soustraction de 26 emprunte
        JC tel_quel
        LDA k
        SUBI 32
        JMP envoie
tel_quel: LDA k
envoie: OUT
        NOP             ; la lettre reste dans la boîte quatre cycles
        NOP
        NOP
        LDI 0
        OUT
relache: IN             ; attend que la touche soit relâchée
        JZ attend
        JMP relache
";

/// L'ordinateur qui reçoit : il affiche chaque octet qui arrive à son clavier. Il laisse à
/// l'autre le temps de vider la boîte avant de la lire.
pub const RECEPTION: &str = "\
; Affiche ce qui arrive dans la boîte aux lettres.
        NOP             ; l'autre ordinateur vide d'abord la boîte
        NOP
        NOP
        NOP
attend: IN
        JZ attend
        OUT
relache: IN             ; attend que la boîte soit vidée
        JZ attend
        JMP relache
";

/// L'ordinateur qui chiffre : chaque touche avance d'un rang (A devient B) avant de partir.
pub const CHIFFREMENT: &str = "\
; Chiffre chaque touche (César, décalage de 1), puis l'envoie suivie d'un 0.
        LDI 0
        OUT             ; la boîte aux lettres part vide
attend: IN
        JZ attend
        ADDI 1          ; le chiffre de César : un rang plus loin
        OUT
        NOP
        NOP
        NOP
        LDI 0
        OUT
relache: IN
        JZ attend
        JMP relache
";

/// L'ordinateur qui déchiffre : chaque octet reçu recule d'un rang, puis s'affiche.
pub const DECHIFFREMENT: &str = "\
; Déchiffre ce qui arrive (un rang en arrière) et l'affiche.
        NOP
        NOP
        NOP
        NOP
attend: IN
        JZ attend
        SUBI 1          ; défait le chiffre de César
        OUT
relache: IN
        JZ attend
        JMP relache
";

/// Pour la mémoire de 256 octets : 24 touches rangées à des adresses éloignées (une mémoire
/// plus petite en confondrait plusieurs), puis rendues à l'envers.
pub const MIROIR: &str = "\
; Lit 24 touches et les range loin les unes des autres, dans les 256 cases de la mémoire,
; puis les affiche à l'envers.
c0 = 0
c1 = 64
c2 = 128
c3 = 192
c4 = 1
c5 = 65
c6 = 129
c7 = 193
c8 = 2
c9 = 66
c10 = 130
c11 = 194
c12 = 3
c13 = 67
c14 = 131
c15 = 195
c16 = 4
c17 = 68
c18 = 132
c19 = 196
c20 = 5
c21 = 69
c22 = 133
c23 = 197
t0:    IN               ; attend une touche
        JZ t0
        STA c0
r0:    IN               ; attend qu'elle soit relâchée
        JZ t1
        JMP r0
t1:    IN               
        JZ t1
        STA c1
r1:    IN               
        JZ t2
        JMP r1
t2:    IN               
        JZ t2
        STA c2
r2:    IN               
        JZ t3
        JMP r2
t3:    IN               
        JZ t3
        STA c3
r3:    IN               
        JZ t4
        JMP r3
t4:    IN               
        JZ t4
        STA c4
r4:    IN               
        JZ t5
        JMP r4
t5:    IN               
        JZ t5
        STA c5
r5:    IN               
        JZ t6
        JMP r5
t6:    IN               
        JZ t6
        STA c6
r6:    IN               
        JZ t7
        JMP r6
t7:    IN               
        JZ t7
        STA c7
r7:    IN               
        JZ t8
        JMP r7
t8:    IN               
        JZ t8
        STA c8
r8:    IN               
        JZ t9
        JMP r8
t9:    IN               
        JZ t9
        STA c9
r9:    IN               
        JZ t10
        JMP r9
t10:    IN              
        JZ t10
        STA c10
r10:    IN              
        JZ t11
        JMP r10
t11:    IN              
        JZ t11
        STA c11
r11:    IN              
        JZ t12
        JMP r11
t12:    IN              
        JZ t12
        STA c12
r12:    IN              
        JZ t13
        JMP r12
t13:    IN              
        JZ t13
        STA c13
r13:    IN              
        JZ t14
        JMP r13
t14:    IN              
        JZ t14
        STA c14
r14:    IN              
        JZ t15
        JMP r14
t15:    IN              
        JZ t15
        STA c15
r15:    IN              
        JZ t16
        JMP r15
t16:    IN              
        JZ t16
        STA c16
r16:    IN              
        JZ t17
        JMP r16
t17:    IN              
        JZ t17
        STA c17
r17:    IN              
        JZ t18
        JMP r17
t18:    IN              
        JZ t18
        STA c18
r18:    IN              
        JZ t19
        JMP r18
t19:    IN              
        JZ t19
        STA c19
r19:    IN              
        JZ t20
        JMP r19
t20:    IN              
        JZ t20
        STA c20
r20:    IN              
        JZ t21
        JMP r20
t21:    IN              
        JZ t21
        STA c21
r21:    IN              
        JZ t22
        JMP r21
t22:    IN              
        JZ t22
        STA c22
r22:    IN              
        JZ t23
        JMP r22
t23:    IN              
        JZ t23
        STA c23
r23:    IN              
        JZ rendu
        JMP r23
rendu:  LDA c23
        OUT
        LDA c22
        OUT
        LDA c21
        OUT
        LDA c20
        OUT
        LDA c19
        OUT
        LDA c18
        OUT
        LDA c17
        OUT
        LDA c16
        OUT
        LDA c15
        OUT
        LDA c14
        OUT
        LDA c13
        OUT
        LDA c12
        OUT
        LDA c11
        OUT
        LDA c10
        OUT
        LDA c9
        OUT
        LDA c8
        OUT
        LDA c7
        OUT
        LDA c6
        OUT
        LDA c5
        OUT
        LDA c4
        OUT
        LDA c3
        OUT
        LDA c2
        OUT
        LDA c1
        OUT
        LDA c0
        OUT
        HLT
";

/// Le programme qui affiche `texte`, caractère par caractère, puis s'arrête.
pub fn afficher(texte: &str) -> String {
    let mut source = String::from("; Affiche un texte.\n");
    for c in texte.chars() {
        let operande = match c {
            '\'' | '\\' | '\n' | '\r' | '\t' => u32::from(c).to_string(),
            c if c.is_ascii() && !c.is_ascii_control() => format!("'{c}'"),
            c => u32::from(c).min(255).to_string(),
        };
        source.push_str(&format!("        LDI {operande}\n        OUT\n"));
    }
    source.push_str("        HLT\n");
    source
}
