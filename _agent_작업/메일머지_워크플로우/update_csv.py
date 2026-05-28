#!/usr/bin/env python3
"""
CSV 데이터 수정 도구 — 사용자 검증 단계에서 OCR 오류 반영

사용법:
    python update_csv.py <csv파일> "<수정명령>"

수정명령 형식:
    "이름 값"                     → 컬럼이 하나면 그 값으로 수정
    "이름 컬럼명 새값"            → 특정 컬럼만 수정
    "이름 키 137.7 몸무게 40.5"  → 여러 컬럼 동시 수정

예시:
    python update_csv.py ocr_결과.csv "단예준 137.7"
                                  → 단예준의 키를 137.7로 (컬럼: 키(숫자))
    python update_csv.py ocr_결과.csv "유예준 시력 0.1 1.2"
                                  → 유예준의 왼쪽시력=0.1, 오른쪽시력=1.2
"""

import csv
import re
import sys


def parse_command(line):
    """'이름 값' 또는 '이름 컬럼 값' 형식 파싱"""
    parts = line.strip().split()
    if len(parts) < 2:
        return None, None, None
    
    name = parts[0]
    
    # '이름 컬럼1 값1 컬럼2 값2' 패턴 (컬럼명이 한글)
    if len(parts) >= 4 and len(parts) % 2 == 0:
        updates = {}
        for i in range(1, len(parts), 2):
            updates[parts[i]] = parts[i + 1]
        return name, updates, 'multi'
    
    # '이름 값' — 단일 값 (컬럼 자동 추론)
    if len(parts) == 2:
        return name, parts[1], 'single'
    
    # '이름 컬럼 값' — 특정 컬럼
    if len(parts) == 3:
        return name, {parts[1]: parts[2]}, 'multi'
    
    return None, None, None


def update_csv(csv_path, command):
    name, updates, mode = parse_command(command)
    if not name:
        print(f"오류: 명령 형식이 잘못됐습니다 → {command}")
        return False
    
    rows = []
    with open(csv_path, encoding='utf-8-sig') as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames
        rows = list(reader)
    
    if not fieldnames:
        print("오류: CSV에 헤더가 없습니다")
        return False
    
    # 이름으로 학생 찾기
    target_rows = [r for r in rows if r.get('이름', '').strip() == name]
    if not target_rows:
        print(f"오류: '{name}' 학생을 찾을 수 없습니다")
        return False
    
    modified = False
    for row in target_rows:
        old_values = dict(row)
        
        if mode == 'single':
            # 단일 값 → 어떤 컬럼을 수정할지 추론
            val = str(updates)
            for col in fieldnames:
                if col == '이름' or col == '번호' or col == '학년' or col == '반':
                    continue
                if row.get(col, '').strip() != val.strip():
                    row[col] = val.strip()
                    modified = True
                    print(f"  {name}: {col} = {val.strip()} (← {old_values.get(col, '')})")
                    break
        
        elif mode == 'multi':
            for col, val in updates.items():
                if col in fieldnames:
                    old = row.get(col, '')
                    row[col] = val.strip()
                    print(f"  {name}: {col} = {val.strip()} (← {old})")
                    modified = True
                else:
                    # '시력' → 왼쪽시력/오른쪽시력 추론
                    if col == '시력':
                        vals = val.strip().split()
                        if len(vals) >= 1:
                            old_l = row.get('왼쪽시력', '')
                            row['왼쪽시력'] = vals[0]
                            print(f"  {name}: 왼쪽시력 = {vals[0]} (← {old_l})")
                        if len(vals) >= 2:
                            old_r = row.get('오른쪽시력', '')
                            row['오른쪽시력'] = vals[1]
                            print(f"  {name}: 오른쪽시력 = {vals[1]} (← {old_r})")
                        modified = True
    
    if modified:
        with open(csv_path, 'w', encoding='utf-8-sig', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(rows)
        print(f"✓ {csv_path} 업데이트 완료")
        return True
    else:
        print("수정된 내용이 없습니다")
        return False


if __name__ == '__main__':
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    
    csv_file = sys.argv[1]
    command = sys.argv[2]
    
    if not os.path.exists(csv_file):
        print(f"오류: CSV 파일 없음 → {csv_file}")
        sys.exit(1)
    
    update_csv(csv_file, command)
